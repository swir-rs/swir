import yaml
import subprocess
import time
import random
import string
import base64
import sys
try:
    postfix= sys.argv[1]
except:
    print('Please provide postfix that was generated when cluster was created')
    sys.exit(1)

if postfix == None:
    print('Please provide postfix that was generated when cluster was created')
    sys.exit(1)


cluster_name = 'swir-demo-si-cluster-'+postfix
#https://github.com/aws/containers-roadmap/issues/632
autoscale_group_name='swir-demo-si-autoscaling-group-'+postfix
capacity_provider_name = 'swir-demo-si-capacity-provider-'+postfix
launch_configuration = 'swir-demo-si-launch-configuration-' + postfix


print('Deleting cluster name ' + cluster_name)
print('Deleting autoscaling group name ' + autoscale_group_name)
print('Deleting launch configuration name ' + launch_configuration)

services = ['library','magazines','books']
tasks = ['swir-library-service','swir-magazines-service','swir-books-service']
files = ['swir-library-service-task-template.yaml','swir-magazines-service-task-template.yaml','swir-books-service-task-template.yaml']

for i,service in enumerate(services):
    file_name = files[i]
    task = tasks[i]
    
    print("Deleting service " + service + " " + task + " " + file_name);
    
    print("Deleting service " + service)
    try:
        subprocess.check_output('aws ecs update-service --cluster ' + cluster_name + ' --service ' + service + ' --desired-count=0',shell=True)
        subprocess.check_output('aws ecs delete-service --cluster ' + cluster_name + ' --service ' + service,shell=True)
    except:
        print('Problem with service ' + service)
        
    output = subprocess.check_output('aws ecs list-tasks --output yaml --cluster ' + cluster_name,shell=True)
    data_loaded = yaml.safe_load(output)
    active_tasks = data_loaded['taskArns']
    for at in active_tasks:
        taskArn = at
        print("Deleting task " + at)
        subprocess.check_output('aws ecs stop-task --cluster ' + cluster_name + ' --task ' + taskArn,shell=True )

    output = subprocess.check_output('aws ecs describe-task-definition --output yaml --task-definition ' + task,shell=True)
    data_loaded = yaml.safe_load(output)
    taskArn = data_loaded['taskDefinition']['taskDefinitionArn']
    print("Deregistering task definition  " + taskArn)
    subprocess.check_output('aws ecs deregister-task-definition --task-definition ' + taskArn,shell=True)

    
print('Deleting auto-scaling group ' + autoscale_group_name)
subprocess.call('aws autoscaling delete-auto-scaling-group --force-delete --auto-scaling-group-name ' + autoscale_group_name,shell=True)
print('Deleting launch configuration '  + launch_configuration)
subprocess.call('aws autoscaling delete-launch-configuration --launch-configuration-name ' + launch_configuration,shell=True)

is_active = False
while not is_active:
    print('Describe cluster');
    output = subprocess.check_output('aws ecs describe-clusters --output yaml --cluster ' + cluster_name,shell=True)
    data_loaded = yaml.safe_load(output)
    registered_instances = data_loaded['clusters'][0]['registeredContainerInstancesCount']
    is_active = (registered_instances==0)
    print('Cluster status '+ data_loaded['clusters'][0]['status'] + ' registered instances ' +  str(registered_instances));
    time.sleep(5)            

subprocess.check_output('aws ecs delete-cluster --cluster ' + cluster_name,shell=True)

