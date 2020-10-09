FROM python:3.8-slim-buster

RUN python -m pip install --upgrade pip && python -m pip install virtualenv grpcio grpcio-tools
COPY *.proto ./
COPY *.py ./
RUN python -m grpc_tools.protoc -I. --python_out=. --grpc_python_out=. common_structs.proto
RUN python -m grpc_tools.protoc -I. --python_out=. --grpc_python_out=. client_api.proto
RUN python -m grpc_tools.protoc -I. --python_out=. --grpc_python_out=. notification_api.proto
ENTRYPOINT ["python3","swir-python-sink.py"]