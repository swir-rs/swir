FROM python:3.8-slim-buster
RUN python -m pip install --upgrade pip 
RUN mkdir -p /swir
COPY *.py /swir/

ENTRYPOINT ["python3","/swir/server.py"]