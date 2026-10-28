#!/usr/bin/env python3
"""Integration checks using a disposable SQLite database and real HTTP server."""
import http.cookiejar
import json
import os
from pathlib import Path
import socket
import subprocess
import tempfile
import time
import urllib.request
import urllib.error

ROOT=Path(__file__).resolve().parents[1]
class Client:
    def __init__(self,base):
        self.base=base
        self.jar=http.cookiejar.CookieJar()
        self.opener=urllib.request.build_opener(urllib.request.HTTPCookieProcessor(self.jar))
    def call(self,path,method='GET',data=None,status=200,guard=True):
        headers={'Content-Type':'application/json'}
        if guard:headers['X-Tamara-Request']='1'
        req=urllib.request.Request(self.base+'/api'+path,data=json.dumps(data).encode() if data is not None else None,method=method,headers=headers)
        try:r=self.opener.open(req)
        except urllib.error.HTTPError as e:r=e
        body=r.read()
        assert r.status==status,(path,r.status,body[:500])
        if r.headers.get('Content-Type','').startswith('application/pdf'):return body
        return json.loads(body) if body else None

def run():
    with tempfile.TemporaryDirectory(prefix='tamara-test-') as tmp:
        sock=socket.socket();sock.bind(('127.0.0.1',0));port=sock.getsockname()[1];sock.close()
        token='test-only-setup-token-354-criteria'
        env={**os.environ,'DATABASE_URL':'sqlite:'+tmp+'/test.sqlite','TAMARA_BIND':f'127.0.0.1:{port}','TAMARA_SETUP_TOKEN':token}
        log=open(Path(tmp)/'server.log','w+')
        server=subprocess.Popen([str(ROOT/'target/debug/tamara')],cwd=ROOT,env=env,stdout=log,stderr=log)
        base=f'http://127.0.0.1:{port}'
        try:
            for _ in range(100):
                try:urllib.request.urlopen(base+'/api/auth/status');break
                except (OSError,urllib.error.URLError):
                    if server.poll() is not None:log.seek(0);raise RuntimeError(log.read())
                    time.sleep(.05)
            admin=Client(base);teacher=Client(base);outsider=Client(base)
            assert admin.call('/auth/status')['setup_required']
            admin.call('/classrooms',status=401)
            credentials={'name':'Admin Test','email':'admin@example.test','password':'test-password-long'}
            admin.call('/auth/register','POST',credentials,status=400)
            admin.call('/auth/register','POST',{**credentials,'registration_token':token},guard=False,status=403)
            a=admin.call('/auth/register','POST',{**credentials,'registration_token':token})
            assert a['is_admin'] is True
            assert not admin.call('/auth/status')['setup_required']
            cookies=list(admin.jar);assert cookies and cookies[0].has_nonstandard_attr('HttpOnly')
            i=admin.call('/admin/invitations','POST',{'label':'Lehrkraft'})
            public=admin.call('/admin/invitations');assert 'token' not in public[0] and 'token_hash' not in public[0]
            c={'name':'Teacher Test','email':'teacher@example.test','password':'another-long-password','registration_token':i['token']}
            b=teacher.call('/auth/register','POST',c);assert not b['is_admin']
            outsider.call('/auth/register','POST',{**c,'email':'reused@example.test'},status=400)
            teacher.call('/admin/invitations',status=403)
            rev=admin.call('/admin/invitations','POST',{'label':'Widerrufen'})
            admin.call('/admin/invitations/'+rev['id'],'DELETE',status=204)
            outsider.call('/auth/register','POST',{**c,'email':'revoked@example.test','registration_token':rev['token']},status=400)
            classroom=admin.call('/classrooms','POST',{'name':'Klasse 2a','school_year':'2026/27'})['id']
            prefix='/classrooms/'+classroom
            assert teacher.call('/classrooms')==[]
            teacher.call(prefix+'/workspace',status=404)
            admin.call(prefix+'/students','POST',{'names':['Mia Müller','Jonas Schmidt','Amira Yılmaz']})
            period=admin.call(prefix+'/periods','POST',{'name':'Herbst','source_period_id':None})['id']
            w=admin.call(prefix+'/workspace');assert len(w['students'])==3 and w['assessments']==[]
            students=w['students'];catalog=admin.call('/criteria')['criteria'];assert len(catalog)==354
            k=catalog[0]['id'];last=catalog[-1]['id']
            endpoint=prefix+'/periods/'+period+'/assessments'
            changes=[{'student_id':students[0]['id'],'criterion_id':k,'value':'yes'}, {'student_id':students[1]['id'],'criterion_id':k,'value':'no'}]
            admin.call(endpoint,'PATCH',{'changes':changes},status=204)
            # A bad row rejects the WHOLE batch, including otherwise valid rows.
            admin.call(endpoint,'PATCH',{'changes':[{'student_id':students[0]['id'],'criterion_id':k,'value':'no'},{'student_id':'bad','criterion_id':k,'value':'yes'}]},status=400)
            assert next(a for a in admin.call(prefix+'/workspace')['assessments'] if a['student_id']==students[0]['id'])['value']=='yes'
            copy=admin.call(prefix+'/periods','POST',{'name':'Winter','source_period_id':period})['id']
            copied=admin.call(prefix+'/workspace?period_id='+copy);assert len(copied['assessments'])==2
            admin.call(prefix+'/periods/'+copy+'/assessments','PATCH',{'changes':[{'student_id':students[0]['id'],'criterion_id':k,'value':None}]},status=204)
            assert len(admin.call(prefix+'/workspace?period_id='+copy)['assessments'])==1
            assert len(admin.call(prefix+'/workspace?period_id='+period)['assessments'])==2
            admin.call(prefix+'/teachers','POST',{'email':c['email']},status=204)
            assert len(teacher.call('/classrooms'))==1
            teacher.call(prefix+'/teachers','POST',{'email':'admin@example.test'},status=403)
            # The same endpoint used by every UI mode supports bulk updates.
            teacher.call(endpoint,'PATCH',{'changes':[{'student_id':s['id'],'criterion_id':last,'value':'yes'} for s in students]},status=204)
            assert len(admin.call(prefix+'/workspace?period_id='+period)['assessments'])==5
            # Cross-class period and student references are rejected.
            other=admin.call('/classrooms','POST',{'name':'Klasse 3b','school_year':'2026/27'})['id']
            admin.call('/classrooms/'+other+'/periods','POST',{'name':'Invalid','source_period_id':period},status=404)
            admin.call('/classrooms/'+other+'/periods/'+period+'/assessments','PATCH',{'changes':changes},status=404)
            outsider.call(prefix+'/periods/'+period+'/students/'+students[0]['id']+'/pdf',status=401)
            # Render every affirmative mark for visual and coordinate checks.
            admin.call(endpoint,'PATCH',{'changes':[{'student_id':students[0]['id'],'criterion_id':r['id'],'value':'yes'} for r in catalog]},status=204)
            pdf=admin.call(prefix+'/periods/'+period+'/students/'+students[0]['id']+'/pdf');assert pdf.startswith(b'%PDF-')
            out=ROOT/'output/pdf';out.mkdir(parents=True,exist_ok=True);(out/'qa-all-criteria.pdf').write_bytes(pdf)
            subprocess.run(['pdfinfo',str(out/'qa-all-criteria.pdf')],check=True,stdout=subprocess.DEVNULL)
            admin.call(prefix+'/teachers/'+b['id'],'DELETE',status=204)
            teacher.call(prefix+'/workspace',status=404)
            teacher.call(prefix+'/periods/'+period+'/students/'+students[0]['id']+'/pdf',status=404)
            # Renaming and deleting a student preserve class boundaries and cascade.
            admin.call(prefix+'/students/'+students[0]['id'],'PATCH',{'name':'Amira Öztürk'},status=204)
            admin.call(prefix+'/students/'+students[2]['id'],'DELETE',status=204)
            assert len(admin.call(prefix+'/workspace')['students'])==2
            teacher.call('/auth/logout','POST',status=204);teacher.call('/auth/me',status=401)
            teacher.call('/auth/login','POST',{'email':c['email'],'password':'wrong'},status=401)
            teacher.call('/auth/login','POST',{'email':c['email'],'password':c['password']})
            assert teacher.call('/auth/me')['id']==b['id']
            attendance=prefix+'/attendance/2026-10-28'
            admin.call(attendance,'PATCH',{'changes':[{'student_id':students[0]['id'],'status':'present'},{'student_id':students[1]['id'],'status':'absent'}]},status=204)
            attendance_before_restart=admin.call(attendance)
            assert len(attendance_before_restart)==2
            # Confirm WAL persistence by restarting the same database.
            server.terminate();server.wait(timeout=5)
            server=subprocess.Popen([str(ROOT/'target/debug/tamara')],cwd=ROOT,env=env,stdout=log,stderr=log)
            for _ in range(100):
                try:
                    assert len(admin.call('/classrooms'))==2;break
                except (OSError,urllib.error.URLError):time.sleep(.05)
            assert admin.call(attendance)==attendance_before_restart
            print('PASS: attendance restart persistence, invite-only signup, admin tokens, login/logout, sharing/revocation, isolation, tri-state grading, atomic bulk updates, period snapshots, student management, persistence, and 20-page PDF export.')
        finally:
            server.terminate();server.wait(timeout=5);log.close()

if __name__=='__main__':run()
