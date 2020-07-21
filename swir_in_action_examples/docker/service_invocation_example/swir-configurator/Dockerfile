FROM ubuntu
RUN mkdir /config /certificates
COPY *.yaml /config/
COPY ./certificates/*.pem /certificates/
CMD ["/bin/bash","-c","cp /config/* /swir_config/;cp /certificates/* certs; ls -l /swir_config/; ls -l certs"]
