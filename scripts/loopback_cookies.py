"""Match browsers' localhost Secure-cookie exception in disposable smoke tests."""
import http.cookiejar
from urllib.parse import urlsplit


class LoopbackCookiePolicy(http.cookiejar.DefaultCookiePolicy):
    def return_ok_secure(self, cookie, request):
        if urlsplit(request.full_url).hostname in ("127.0.0.1", "localhost", "::1"):
            return True
        return super().return_ok_secure(cookie, request)


def loopback_cookie_jar():
    return http.cookiejar.CookieJar(policy=LoopbackCookiePolicy())
