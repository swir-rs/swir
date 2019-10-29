#!/bin/bash
rm rustycar.CA.*
rm -r demoCA
mkdir demoCA
mkdir demoCA/newcerts
touch demoCA/index.txt
echo deadbeef > demoCA/serial
echo 123456 > pass.txt
echo "Genrating CA Key"
openssl genrsa  -passout file:pass.txt -des3 -out rustycar.CA.key 4096
openssl rsa -in rustycar.CA.key -out rustycar.CA.pem -outform PEM -passin file:pass.txt

#echo "CERT"
#
#openssl req -new -x509 -key rustycar.CA.key -out rustycar.CA.crt.pem -outform pem

#echo "DER"
#
#openssl x509 -in rustycar.CA.crt.pem -inform pem -out rustycar.CA.key.pem.der -outform der


echo "Generating CA CSR"
openssl req -verbose -new -key rustycar.CA.pem -out rustycar.client.csr -sha256 -subj "/C=IE/ST=Leinster/L=Dublin/O=ESS/OU=Rustycar/CN=Rustycar CA" -passin file:pass.txt
echo "Signing CA CSR"
openssl ca -verbose -extensions v3_ca -keyfile rustycar.CA.pem -out rustycar.client.signed.crt -selfsign -passin file:pass.txt -md sha256 -enddate 330630235959Z -infiles rustycar.client.csr
cp demoCA/newcerts/DEADBEEF.pem demoCA/cacert.pem

#Add this cert to Trusted Authorities list on your browser