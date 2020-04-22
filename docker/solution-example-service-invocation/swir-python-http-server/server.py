
"""
Very simple HTTP server in python for logging requests
Usage::
    ./server.py [<port>]
"""
from http.server import BaseHTTPRequestHandler, HTTPServer
import logging
from io import BytesIO

class S(BaseHTTPRequestHandler):

    def do_GET(self):
        content_length = int(self.headers['Content-Length']) # <--- Gets the size of data
        post_data = self.rfile.read(content_length) # <--- Gets the data itself
        logging.info("GET, %s Headers:%s Body:%s", str(self.path), str(self.headers).strip().replace('\n',';'), post_data.decode('utf-8'))
        self.send_response(200)
        self.end_headers()
        response = BytesIO()
        response.write(post_data)
        self.wfile.write(response.getvalue())

    def do_DELETE(self):
        content_length = int(self.headers['Content-Length']) # <--- Gets the size of data
        post_data = self.rfile.read(content_length) # <--- Gets the data itself
        logging.info("DELETE, %s Headers:%s Body:%s", str(self.path), str(self.headers).strip().replace('\n',';'), post_data.decode('utf-8'))
        self.send_response(200)
        self.end_headers()
        response = BytesIO()
        response.write(post_data)
        self.wfile.write(response.getvalue())

    def do_PUT(self):
        content_length = int(self.headers['Content-Length']) # <--- Gets the size of data
        post_data = self.rfile.read(content_length) # <--- Gets the data itself
        logging.info("PUT, %s Headers:%s Body:%s", str(self.path), str(self.headers).strip().replace('\n',';'), post_data.decode('utf-8'))
        self.send_response(200)
        self.end_headers()
        response = BytesIO()
        response.write(post_data)
        self.wfile.write(response.getvalue())                


    def do_POST(self):
        content_length = int(self.headers['Content-Length']) # <--- Gets the size of data
        post_data = self.rfile.read(content_length) # <--- Gets the data itself
        logging.info("POST, %s Headers:%s Body:%s", str(self.path), str(self.headers).strip().replace('\n',';'), post_data.decode('utf-8'))
        self.send_response(200)
        self.end_headers()
        response = BytesIO()
        response.write(post_data)
        self.wfile.write(response.getvalue())

def run(server_class=HTTPServer, handler_class=S, port=8080):
    logging.basicConfig(level=logging.INFO)
    server_address = ('', port)
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
    run(port=int(port))
