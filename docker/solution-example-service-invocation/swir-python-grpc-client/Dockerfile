FROM python:3.8-buster
RUN python -m pip install --upgrade pip && python -m pip install grpcio grpcio-tools
RUN mkdir -p /swir
COPY *.py /swir/

ENTRYPOINT ["python3","/swir/swir-python-client.py"]	