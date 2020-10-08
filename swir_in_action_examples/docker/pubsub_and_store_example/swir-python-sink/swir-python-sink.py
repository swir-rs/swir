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
from concurrent import futures

import random
import logging
import uuid
import grpc
import os
import queue
import threading
import grpc
import client_api_pb2
import client_api_pb2_grpc
import notification_api_pb2_grpc
import time
import json

logger = logging.getLogger('swir')

class NotificationApiServicer(notification_api_pb2_grpc.NotificationApiServicer):

    def __init__(self, incoming_queue):
        self.queue = incoming_queue

    def subscriptionNotification(self, request, context):
        logger.debug("Subscription : %s" % request)
        self.queue.put(request)
        return client_api_pb2.SubscribeResponse(correlation_id=request.correlation_id,status="Ok")
        



def receiver(pub_sub_stub,topic):
    while True:
        try:
            subscribe = client_api_pb2.SubscribeRequest(
                correlation_id=str(uuid.uuid4()),
                topic=topic
            )
            res = pub_sub_stub.Subscribe(subscribe)
            logger.info("Subscribed : %s  " % (res.correlation_id))
            return
        except:
            logger.error("Can't connect to sidecar")
            

def processor(incoming_queue,outgoing_queue,persistence_stub,database_name):
    while True:
        msg = incoming_queue.get()        
        if msg is None:
            break        
        logger.info("Consumed : %s %s" %(msg.correlation_id,msg.payload))
        payload = json.loads(msg.payload)
        key= str(payload['counter'])
        delete = client_api_pb2.DeleteRequest(
            correlation_id=msg.correlation_id,
            database_name=database_name,
            key=key
            )
        res = persistence_stub.Delete(delete)
        logger.info("Deleted from store : %s %s %s" %(key,res.payload,res.status))
    
                

def run():
    logger.info("Starting gRPC Sink")
    
    incoming_queue = queue.Queue()
    outgoing_queue = queue.Queue()
    subscribe_topic = os.environ['subscribe_topic']
    sidecar = os.environ['sidecar']
    database_name = os.environ['client_database_name']

    logger.info("Sidecar is %s" % sidecar)
    logger.info("Subscribe topic is %s" % subscribe_topic)

    server = grpc.server(futures.ThreadPoolExecutor(max_workers=10))
    notification_api_pb2_grpc.add_NotificationApiServicer_to_server(
        NotificationApiServicer(incoming_queue), server)
    server.add_insecure_port('0.0.0.0:50053')
    server.start()
    
    
    with grpc.insecure_channel(sidecar) as channel:
        threads = []
        pub_sub_api_stub = client_api_pb2_grpc.PubSubApiStub(channel)
        persistence_api_stub = client_api_pb2_grpc.PersistenceApiStub(channel)
        t1 = threading.Thread(target=receiver, args=[pub_sub_api_stub,subscribe_topic] )
        t2 = threading.Thread(target=processor, args=[incoming_queue, outgoing_queue,persistence_api_stub,database_name])
        t1.start()
        threads.append(t1)
        t2.start()
        threads.append(t2)


        for t in threads:
            t.join()
            
if __name__ == '__main__':
    logger.setLevel(logging.INFO)
    console = logging.StreamHandler()
    formatter = logging.Formatter('%(asctime)s - %(process)d - %(funcName)s:%(lineno)d - %(levelname)s - %(message)s')
    console.setFormatter(formatter)
    logger.addHandler(console)
    run()
