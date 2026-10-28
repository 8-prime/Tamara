#!/usr/bin/env python3
"""Check a built image's HTTP serving, non-root user, shutdown, and SQLite volume.

Usage: python3 scripts/container-smoke-test.py [image-tag]
Requires Docker; creates and removes an isolated container and volume.
"""
import http.cookiejar
import json
import re
import subprocess
import sys
import time
import urllib.error
import urllib.request
import uuid


def docker(*args, check=True):
    result = subprocess.run(
        ["docker", *args], capture_output=True, text=True
    )
    if check and result.returncode:
        raise RuntimeError(f"docker {' '.join(args)} failed:\n{result.stdout}{result.stderr}")
    return (result.stdout + (result.stderr if args[0] == "logs" else "")).strip()


def run(image):
    name = "tamara-smoke-" + uuid.uuid4().hex[:12]
    volume = name + "-data"
    setup_token = "container-test-setup-token-" + uuid.uuid4().hex
    base = ""
    jar = http.cookiejar.CookieJar()
    client = urllib.request.build_opener(urllib.request.HTTPCookieProcessor(jar))

    def call(path, method="GET", data=None, expected=200):
        request = urllib.request.Request(
            base + path,
            data=json.dumps(data).encode() if data is not None else None,
            method=method,
            headers={"Content-Type": "application/json", "X-Tamara-Request": "1"},
        )
        try:
            response = client.open(request, timeout=5)
        except urllib.error.HTTPError as error:
            response = error
        with response:
            body = response.read()
            assert response.status == expected, (path, response.status, body[:200])
            return body, response.headers

    def wait_until_ready():
        for _ in range(60):
            try:
                call("/api/auth/status")
                return
            except (OSError, urllib.error.URLError):
                if docker("inspect", "--format", "{{.State.Running}}", name) != "true":
                    raise RuntimeError("Container exited before becoming ready")
                time.sleep(0.5)
        raise RuntimeError("Container did not become ready")

    docker("volume", "create", volume)
    try:
        docker(
            "run", "--detach", "--name", name,
            "--publish", "127.0.0.1::3000",
            "--mount", f"source={volume},target=/data",
            "--env", f"TAMARA_SETUP_TOKEN={setup_token}", image,
        )
        address = docker("port", name, "3000/tcp").splitlines()[0]
        base = "http://" + address
        wait_until_ready()
        assert docker("exec", name, "id", "-u") != "0"
        page, headers = call("/")
        assert "text/html" in headers["Content-Type"] and b"Tamara" in page
        assets = re.findall(rb'(?:src|href)="(/assets/[^\"]+)"', page)
        assert assets, "No compiled frontend assets found"
        for asset in assets:
            content, headers = call(asset.decode())
            assert content and "text/html" not in headers.get("Content-Type", "")
        assert call("/classrooms/example")[0] == page
        call("/assets/missing.js", expected=404)
        _, headers = call("/api/missing", expected=404)
        assert headers["Content-Type"].startswith("application/json")
        teacher, _ = call("/api/auth/register", "POST", {
            "name": "Container Admin", "email": "container@example.test",
            "password": "container-test-password-long", "registration_token": setup_token,
        })
        teacher = json.loads(teacher)
        assert teacher["is_admin"]
        call("/api/classrooms", "POST", {"name": "Container Class", "school_year": "2026/27"})
        docker("stop", "--timeout", "10", name)
        assert docker("inspect", "--format", "{{.State.ExitCode}}", name) == "0", "SIGTERM was not graceful"
        # Recreate the container with the same volume to check durable data.
        docker("rm", name)
        docker(
            "run", "--detach", "--name", name,
            "--publish", "127.0.0.1::3000",
            "--mount", f"source={volume},target=/data", image,
        )
        base = "http://" + docker("port", name, "3000/tcp").splitlines()[0]
        wait_until_ready()
        assert not json.loads(call("/api/auth/status")[0])["setup_required"]
        # CookieJar scopes cookies by host, so the new published port is fine.
        assert json.loads(call("/api/auth/me")[0])["id"] == teacher["id"]
        classrooms = json.loads(call("/api/classrooms")[0])
        assert len(classrooms) == 1 and classrooms[0]["name"] == "Container Class"
        print("PASS: compiled frontend/assets/API, non-root runtime, graceful SIGTERM, and persistent accounts/sessions/classrooms across container replacement.")
    except Exception:
        print(docker("logs", name, check=False), file=sys.stderr)
        raise
    finally:
        docker("rm", "--force", name, check=False)
        docker("volume", "rm", volume, check=False)


if __name__ == "__main__":
    run(sys.argv[1] if len(sys.argv) > 1 else "tamara:local")
