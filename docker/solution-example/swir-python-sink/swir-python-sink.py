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
import time

logger = logging.getLogger('swir')

def receiver(queue,client_api_stub,topic):
    while True:
        try:
            subscribe = client_api_pb2.SubscribeRequest(
                correlation_id=str(uuid.uuid4()),
                topic=topic
            )

            messages = client_api_stub.Subscribe(subscribe)
            for message in messages:
                logger.debug("Subscription : %s" % message)
                queue.put(message)        
        except:
            logger.error("Can't connect to sidecar")            
        time.sleep(5)


def processor(incoming_queue,outgoing_queue):
    while True:
        msg = incoming_queue.get()
        if msg is None:
            break
        logger.info("Consumed : %s %s" %(msg.correlation_id,msg.payload))
    
                

def run():
    logger.info("Starting gRPC Sink")
    
    incoming_queue = queue.Queue()
    outgoing_queue = queue.Queue()
    subscribe_topic = os.environ['subscribe_topic']
    sidecar = os.environ['sidecar']

    logger.info("Sidecar is %s" % sidecar)
    logger.info("Subscribe topic is %s" % subscribe_topic)
    
    with grpc.insecure_channel(sidecar) as channel:
        threads = []
        client_api_stub = client_api_pb2_grpc.ClientApiStub(channel)
        t1 = threading.Thread(target=receiver, args=[incoming_queue, client_api_stub,subscribe_topic] )
        t2 = threading.Thread(target=processor, args=[incoming_queue, outgoing_queue])
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
