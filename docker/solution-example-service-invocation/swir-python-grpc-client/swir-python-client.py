# Copyright 2020 SWIR authors.
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.
"""The Python implementation of an example SWIR gRPC Sink."""

from __future__ import print_function

import random
import logging
import uuid
import grpc
import os
import queue
import threading
import client_api_pb2
import client_api_pb2_grpc
import common_structs_pb2
import common_structs_pb2_grpc
import time
import json
import random, string
import base64

logger = logging.getLogger('swir')

def randomword(length):
    letters = string.ascii_lowercase
    return ''.join(random.choice(letters) for i in range(length))      

def run():
    logger.info("Starting gRPC Client")

    sidecar = os.environ['sidecar']
    service_names = os.environ['service_names'].split(',')

    logger.info("Sidecar is %s" % sidecar)
    logger.info("Service names is %s" % str(service_names))
    with grpc.insecure_channel(sidecar) as channel:
        service_invocation_api_stub = client_api_pb2_grpc.ServiceInvocationApiStub(channel)        
        while True:
            for service_name in service_names:                
                corr_id = str(uuid.uuid4())
                payload = randomword(50).encode()
                service_id = randomword(10).encode()
                logger.info("Sending %s %s %s" %(service_name, corr_id, payload));
                for i in range(4):
                    headers = {'Content-type':'application/json'}
                    headers['x-client-corr-id']=str(uuid.uuid4())
                    headers['Host']='192.168.2.45:8090'
                    response = service_invocation_api_stub.Invoke(common_structs_pb2.InvokeRequest(correlation_id=corr_id,method=i,service_name=service_name,headers=headers,request_target="/"+service_name+"?"+service_name+"Id=1223434",payload=payload))
                if response.result.status == common_structs_pb2.InvokeStatus.Error:
                    logger.warning("Received %s %s %s " %(service_name, response.correlation_id, str(response.result).replace('\n',' ')))
                else:
                    logger.info("Received %s %s %s %s " %(service_name, response.correlation_id, str(response.result).replace('\n',' '),str(response.payload)))
                time.sleep(5)                                                                        
            
if __name__ == '__main__':
    logger.setLevel(logging.INFO)
    console = logging.StreamHandler()
    formatter = logging.Formatter('%(asctime)s - %(process)d - %(funcName)s:%(lineno)d - %(levelname)s - %(message)s')
    console.setFormatter(formatter)
    logger.addHandler(console)
    run()
