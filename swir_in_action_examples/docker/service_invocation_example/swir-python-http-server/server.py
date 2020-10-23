
"""
Very simple HTTP server in python for logging requests
Usage::
    ./server.py [<port>]
"""
from http.server import BaseHTTPRequestHandler, HTTPServer
import logging
from io import BytesIO
import random, string
import socket

def randomword(length):
    letters = string.ascii_lowercase
    return ''.join(random.choice(letters) for i in range(length))      
instance_id = "instance-"+socket.getfqdn()
class S(BaseHTTPRequestHandler):
    
    def do_GET(self):
        content_length = int(self.headers['Content-Length']) # <--- Gets the size of data
        post_data = self.rfile.read(content_length) # <--- Gets the data itself
        logging.info("GET, %s Headers:%s Body:%s", str(self.path), str(self.headers).strip().replace('\n',';'), post_data.decode('utf-8'))
        self.send_response(200)
        self.send_header('x-swir-greetings','this is a custom header from the farway service behind two swirs')
        self.end_headers()
        response = BytesIO()
        p = str(instance_id + "--")
        response.write(p.encode())        
        response.write(post_data)
        self.wfile.write(response.getvalue())

    def do_DELETE(self):
        content_length = int(self.headers['Content-Length']) # <--- Gets the size of data
        post_data = self.rfile.read(content_length) # <--- Gets the data itself
        logging.info("DELETE, %s Headers:%s Body:%s", str(self.path), str(self.headers).strip().replace('\n',';'), post_data.decode('utf-8'))
        self.send_response(200)
        self.send_header('x-swir-greetings','this is a custom header from the farway service behind two swirs')
        self.end_headers()
        response = BytesIO()
        p = str(instance_id + "--")
        response.write(p.encode())
        response.write(post_data)
        self.wfile.write(response.getvalue())

    def do_PUT(self):
        content_length = int(self.headers['Content-Length']) # <--- Gets the size of data
        post_data = self.rfile.read(content_length) # <--- Gets the data itself
        logging.info("PUT, %s Headers:%s Body:%s", str(self.path), str(self.headers).strip().replace('\n',';'), post_data.decode('utf-8'))
        self.send_response(200)
        self.send_header('x-swir-greetings','this is a custom header from the farway service behind two swirs')
        self.end_headers()
        response = BytesIO()
        p = str(instance_id + "--")
        response.write(p.encode())
        response.write(post_data)        
        self.wfile.write(response.getvalue())                


    def do_POST(self):
        content_length = int(self.headers['Content-Length']) # <--- Gets the size of data
        post_data = self.rfile.read(content_length) # <--- Gets the data itself
        logging.info("POST, %s Headers:%s Body:%s", str(self.path), str(self.headers).strip().replace('\n',';'), post_data.decode('utf-8'))
        self.send_response(200)
        self.send_header('x-swir-greetings','this is a custom header from the farway service behind two swirs')
        self.end_headers()
        response = BytesIO()
        p = str(instance_id + "--")
        response.write(p.encode())
        response.write(post_data)                
        self.wfile.write(response.getvalue())

def run(server_class=HTTPServer, handler_class=S, port=8080,ip='127.0.0.1'):
    logging.basicConfig(level=logging.INFO)
    server_address = (ip, port)
    httpd = server_class(server_address, handler_class)
    logging.info('Starting httpd...\n')
    try:
        httpd.serve_forever()
    except KeyboardInterrupt:
        pass
    httpd.server_close()
    logging.info('Stopping httpd...\n')

if __name__ == '__main__':
    import os
    port = os.environ['port']
    ip = os.environ['ip']
    run(port=int(port), ip)
