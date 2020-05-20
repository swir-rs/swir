FROM python:3.8-slim-buster
RUN python -m pip install --upgrade pip && python -m pip install grpcio grpcio-tools
RUN mkdir -p /swir
RUN mkdir -p /certs
COPY *.py /swir/

ENTRYPOINT ["python3","/swir/swir-python-client.py"]	