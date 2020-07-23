#!/bin/bash
minikube_version='v1.11.0'
minikube_version_major='1'
minikube_version_minor='11'
kubectl_version='v1.17.0'
kubectl_version_major='1'
kubectl_version_minor='17'

helm_version='v3.2.4'

#Check system requirements
x=$(nproc)
error=0
printf "Checking CPUs\n"
if [[ $x/2 -le 3 ]]; then
    printf "You have only $x cores. We would recommend dedicating at least 4 CPUs to Minikube\n"
    error=1
else
    printf "CPUs OK .... $x\n"
fi

printf "Checking Memory\n"
x=$(free -gh | grep Mem | awk '{ print $2 }')
x=${x::-2}
printf -v int '%d' $x 2>/dev/null
x=$int
if [[ $x -le 8 ]]; then
    printf "You have only $x Gb of total memory. We would recommend dedicating at least 4Gb to Minikube\n"
    error=1
else
    printf "Memory OK .... $x\n"
fi

# Minikube check 
x=$(minikube version | grep v1 | awk '{ print $3}')
minor=${x::-2}
minor=${minor:3}
major=${x:5}
major=${major:1}
printf "Checking Minikube Version\n"
if [[ $minor -le $minikube_version_minor && $major -eq $minikube_version_major ]]; then
    printf "Minikube not installed or wrong version installed $x. We recommend using version $minikube_version\n"
    error=1
else
    printf "Minikube OK .... $x\n"    
fi    


# Kubectl check 
x=$(kubectl version --client=true --short=true |  awk '{ print $3}')
minor=${x::-2}
minor=${minor:3}
major=${x:5}
major=${major:1}


printf "Checking Kubectl Version\n"
if [[ $minor -le $kubectl_version_minor && $major -eq $kubectl_version_major ]]; then
    printf "Kubectl not installed or wrong version installed $x. We recommend using version $kubectl_version\n"
    error=1
else
    printf "Kubectl OK .... $x\n"    
fi

# Helm check
x=$(helm version | helm version | awk '{ print $1 }')
x=${x::-2}
x=${x:27}
printf "Checking Helm Version\n"
if [[ $x != $helm_version ]]; then
    printf "Helm not installed or wrong version installed $x. We recommend using version $helm_version\n"
    error=1
else
    printf "Helm OK  .... $x\n"        
fi

if [[ $error -eq 1 ]]; then
   echo ""
   echo ""
   echo "************************************"
   echo ""
   echo ""
   echo "Not all conditions are met. "
   echo ""
   echo "Examples might be unstable or your PC could become unresponsive"
   echo ""
   echo ""   
   echo "************************************"
   echo ""
   echo ""   
   sleep 10
fi
