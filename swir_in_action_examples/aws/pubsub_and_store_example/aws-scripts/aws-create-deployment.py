import yaml
import subprocess
import time
import random
import string
import base64
import sys
import re

try:
    aws_account = sys.argv[1]
except:
    print('Please provide valid AWS account')
    sys.exit (1)    

try:
    aws_region = sys.argv[2]
except:
    print('Please provide valid AWS region')
    sys.exit (1)


postfix = ''.join(random.choices(string.ascii_uppercase + string.digits, k=10))

cluster_name = 'swir-demo-cluster-'+postfix
#https://github.com/aws/containers-roadmap/issues/632
autoscale_group_name='swir-demo-autoscaling-group-'+postfix
capacity_provider_name = 'swir-demo-capacity-provider-'+postfix
launch_configuration = 'swir-demo-launch-configuration-' + postfix

print('Postfix ' + postfix)
print('Cluster name ' + cluster_name)
print('Cluster capacity provider name ' + capacity_provider_name)
print('Cluster autoscaling group name ' + autoscale_group_name)
print('Cluster launch configuration name ' + launch_configuration)

with open('create-launch-configuration-template.yaml') as f:
    print('Create launch configuration')
    data_loaded = yaml.safe_load(f)
    data_loaded['LaunchConfigurationName']=launch_configuration
    data_loaded['ImageId']='ami-0851c53aff84212c3' #ami-035966e8adab4aaad
#    data_loaded['SecurityGroups']=['sg-0e81baff8bccf2ce3']

# https://stackoverflow.com/questions/35202993/how-can-i-connect-my-autoscaling-group-to-my-ecs-cluster
    data_loaded['UserData']='#!/bin/bash\necho ECS_CLUSTER=' + cluster_name + ' >> /etc/ecs/ecs.config;echo ECS_BACKEND_HOST= >> /etc/ecs/ecs.config;'
    out = open('create-launch-configuration-template.filled.yaml', 'w')
    out.write(yaml.dump(data_loaded))
    out.close()
    subprocess.call('aws autoscaling create-launch-configuration --cli-input-yaml file://create-launch-configuration-template.filled.yaml',shell=True)
    
with open('create-auto-scaling-group-template.yaml') as f:
    data_loaded = yaml.safe_load(f)
    data_loaded['AutoScalingGroupName']=autoscale_group_name
    data_loaded['LaunchConfigurationName']=launch_configuration
    data_loaded['MinSize']=5
    data_loaded['DesiredCapacity']=5
    data_loaded['MaxSize']=5
    data_loaded['AvailabilityZones']=[aws_region+'b', aws_region+'a']

#    data_loaded['VPCZoneIdentifier']='subnet-08b2e680fa69be06f,subnet-0739c651539cf6bb7'
    out = open('create-auto-scaling-group-template.filled.yaml', 'w')
    out.write(yaml.dump(data_loaded))
    out.close()
    print('Create autoscaling groups')
    subprocess.call('aws autoscaling create-auto-scaling-group --cli-input-yaml file://create-auto-scaling-group-template.filled.yaml',shell=True)    
    print('Describe autoscaling groups')
    output = subprocess.check_output('aws autoscaling describe-auto-scaling-groups --auto-scaling-group-name '+ autoscale_group_name + ' --output yaml',shell=True)
    data_loaded = yaml.safe_load(output)
    auto_scaling_group_arn = data_loaded['AutoScalingGroups'][0]['AutoScalingGroupARN']


required_instances = 5

with open('create-cluster-template.yaml') as f:
    data_loaded = yaml.safe_load(f)
    data_loaded['clusterName']=cluster_name
#    data_loaded['capacityProviders'][0]=capacity_provider_name
    data_loaded['defaultCapacityProviderStrategy']=[]
    data_loaded['capacityProviders']=[]
    # data_loaded['defaultCapacityProviderStrategy'][0]['capacityProvider']=capacity_provider_name
    # data_loaded['defaultCapacityProviderStrategy'][0]['weight']=1
    # data_loaded['defaultCapacityProviderStrategy'][0]['base']=1
    data_loaded['tags'][0]['key']='swir-cluster'

    out = open('create-cluster-template.filled.yaml', 'w')
    out.write(yaml.dump(data_loaded))
    out.close()
    try:
        print('Create cluster');
        output = subprocess.check_output('aws ecs create-cluster --cli-input-yaml file://create-cluster-template.filled.yaml',shell=True)
    except:
        print('Error when creating cluster')
    
    is_active = False
    while not is_active:
        print('Describe cluster');
        output = subprocess.check_output('aws ecs describe-clusters --output yaml --cluster ' + cluster_name,shell=True)
        data_loaded = yaml.safe_load(output)
        registered_instances = data_loaded['clusters'][0]['registeredContainerInstancesCount']
        is_active = (data_loaded['clusters'][0]['status'] == 'ACTIVE' and registered_instances>=required_instances)
        print('Cluster status '+ data_loaded['clusters'][0]['status'] + ' registered instances ' +  str(registered_instances) + "/"+ str(required_instances));
        time.sleep(5)            

output = subprocess.check_output('aws iam get-role --output yaml --role-name swirEcsTaskExecutionRole',shell=True)
data_loaded = yaml.safe_load(output)
roleArn = data_loaded['Role']['Arn']

services = ['incoming','orders','inventory','billing','shipments']
tasks = ['swir-order-generator','swir-order-processor','swir-inventory-processor','swir-billing-processor','swir-shipments-sink']
files = ['swir-order-generator-task-template','swir-order-processor-task-template','swir-inventory-processor-task-template','swir-billing-processor-task-template','swir-shipment-sink-task-template']

for i,service in enumerate(services):
    file_name = files[i]
    task = tasks[i]
    print("Creating service " + service + " " + task + " " + file_name);

    ff = file_name+'-filled.yaml'
    with open(file_name+'.yaml') as f:
        f = f.read()
        f = re.sub('<aws_region>',aws_region,f)
        f = re.sub('<aws_account>',aws_account,f)        
        data_loaded = yaml.safe_load(f)
        data_loaded['taskRoleArn']=roleArn
        data_loaded['executionRoleArn']=roleArn      
        out = open(ff, 'w')
        out.write(yaml.dump(data_loaded))
        out.close()
    
    subprocess.check_output('aws ecs register-task-definition --cli-input-yaml file://' + ff,shell=True)            
    output = subprocess.check_output('aws ecs describe-task-definition --output=yaml --task ' + task,shell=True)
    data_loaded = yaml.safe_load(output)
    task_definition_arn = data_loaded['taskDefinition']['taskDefinitionArn']
    with open('create-service-template.yaml') as f:
        data_loaded = yaml.safe_load(f)
        data_loaded['cluster']=cluster_name
        data_loaded['serviceName']=service
        data_loaded['taskDefinition']=task_definition_arn
        data_loaded['capacityProviderStrategy']=[]
        data_loaded['clientToken']=''.join(random.choices(string.ascii_uppercase + string.digits, k=10))
        ff = 'create-service-template.filled.'+service+'.yaml'
        out = open(ff, 'w')
        out.write(yaml.dump(data_loaded))
        out.close()
        try:
            subprocess.check_output('aws ecs create-service --cli-input-yaml file://' + ff ,shell=True)
        except:
            print('Unable to create service ' + service)
              
#    print(yaml.dump(data_loaded))

