In order to generate client you could use one of [OpenAPI generators](https://github.com/OpenAPITools/openapi-generator).
For example this will generate a Java client library :
```
docker run --rm -v ${PWD}:/local openapitools/openapi-generator-cli generate -i local/client_api.yaml -g java --library native --additional-properties groupId=rs.swir,artifactId=swir-client,modelPackage=rs.swir.client.model,apiPackage=rs.swir.client.api,hideGenerationTimestamp=true -o local/swir-client
```
