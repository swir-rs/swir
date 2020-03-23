FROM ubuntu
RUN mkdir /config
COPY *.yaml /config/
CMD ["/bin/bash","-c","cp /config/* /swir_config/; ls -l /swir_config/"]

