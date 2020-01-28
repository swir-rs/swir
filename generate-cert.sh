#!/bin/bash
rm swir.CA.*
rm -r demoCA
mkdir demoCA
mkdir demoCA/newcerts
touch demoCA/index.txt
echo deadbeef > demoCA/serial
echo 123456 > pass.txt
echo "Genrating CA Key"
openssl genrsa  -passout file:pass.txt -des3 -out swir.CA.key 4096
openssl rsa -in swir.CA.key -out swir.CA.pem -outform PEM -passin file:pass.txt

#echo "CERT"
#
#openssl req -new -x509 -key swir.CA.key -out swir.CA.crt.pem -outform pem

#echo "DER"
#
#openssl x509 -in swir.CA.crt.pem -inform pem -out swir.CA.key.pem.der -outform der


echo "Generating CA CSR"
openssl req -verbose -new -key swir.CA.pem -out swir.client.csr -sha256 -subj "/C=IE/ST=Leinster/L=Dublin/O=ESS/OU=swir/CN=Swir CA" -passin file:pass.txt
echo "Signing CA CSR"
openssl ca -verbose -extensions v3_ca -keyfile swir.CA.pem -out swir.client.signed.crt -selfsign -passin file:pass.txt -md sha256 -enddate 330630235959Z -infiles swir.client.csr
cp demoCA/newcerts/DEADBEEF.pem demoCA/cacert.pem
