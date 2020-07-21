FROM ubuntu:19.10
RUN mkdir /config
COPY *.yaml /config/
CMD ["/bin/bash","-c","cp /config/* /swir_config/; ls -l /swir_config/"]

