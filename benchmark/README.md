# Running Benchmarks
Forked from (Narwhal) [https://github.com/asonnino/narwhal].

This document explains how to benchmark the codebase and read benchmarks' results. It also provides a step-by-step tutorial to run benchmarks on [Amazon Web Services (AWS)](https://aws.amazon.com) accross multiple data centers (WAN).

## Local Benchmarks
When running benchmarks, the codebase is automatically compiled with the feature flag `benchmark`. This enables the node to print some special log entries that are then read by the python scripts and used to compute performance. These special log entries are clearly indicated with comments in the code: make sure to not alter them (otherwise the benchmark scripts will fail to interpret the logs).

### Parametrize the benchmark
After cloning the repo and [installing all dependencies](https://github.com/asonnino/narwhal#quick-start), you can use [Fabric](http://www.fabfile.org/) to run benchmarks on your local machine.  Locate the task called `local` in the file [fabfile.py](https://github.com/asonnino/narwhal/blob/master/benchmark/fabfile.py):
```python
@task
def local(ctx):
    ...
```
The task specifies two types of parameters, the *benchmark parameters* and the *nodes parameters*. The benchmark parameters look as follows:
```python
bench_params = {
    'nodes': 4,
    'workers': 1,
    'rate': 50_000,
    'tx_size': 512,
    'faults': 0,
    'duration': 20,
}
```

### Run the benchmark
Once you specified both `bench_params` and `node_params` as desired, run:
```
$ fab local
```
This command first recompiles your code in `release` mode (and with the `benchmark` feature flag activated), thus ensuring you always benchmark the latest version of your code. This may take a long time the first time you run it. It then generates the configuration files and keys for each node, and runs the benchmarks with the specified parameters. 

## AWS Benchmarks
This repo integrates various python scripts to deploy and benchmark the codebase on [Amazon Web Services (AWS)](https://aws.amazon.com). They are particularly useful to run benchmarks in the WAN, across multiple data centers. This section provides a step-by-step tutorial explaining how to use them.

### Step 1. Set up your AWS credentials
Set up your AWS credentials to enable programmatic access to your account from your local machine. These credentials will authorize your machine to create, delete, and edit instances on your AWS account programmatically. First of all, [find your 'access key id' and 'secret access key'](https://docs.aws.amazon.com/cli/latest/userguide/cli-configure-quickstart.html#cli-configure-quickstart-creds). Then, create a file `~/.aws/credentials` with the following content:
```
[default]
aws_access_key_id = YOUR_ACCESS_KEY_ID
aws_secret_access_key = YOUR_SECRET_ACCESS_KEY
```
Do not specify any AWS region in that file as the python scripts will allow you to handle multiple regions programmatically.

### Step 2. Add your SSH public key to your AWS account
You must now [add your SSH public key to your AWS account](https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/ec2-key-pairs.html). This operation is manual (AWS exposes little APIs to manipulate keys) and needs to be repeated for each AWS region that you plan to use. Upon importing your key, AWS requires you to choose a 'name' for your key; ensure you set the same name on all AWS regions. This SSH key will be used by the python scripts to execute commands and upload/download files to your AWS instances.
If you don't have an SSH key, you can create one using [ssh-keygen](https://www.ssh.com/ssh/keygen/):
```
$ ssh-keygen -f ~/.ssh/aws
```

### Step 3. Configure the testbed
The file [settings.json](https://github.com/asonnino/narwhal/blob/master/benchmark/settings.json) (located in [narwhal/benchmarks](https://github.com/asonnino/narwhal/blob/master/benchmark)) contains all the configuration parameters of the testbed to deploy. Its content looks as follows:
```json
{
    "key": {
        "name": "aws",
        "path": "/absolute/key/path"
    },
    "port": 5000,
    "repo": {
        "name": "delphi-rs",
        "url": "https://github.com/akhilsb/delphi-rs.git",
        "branch": "master"
    },
    "instances": {
        "type": "t2.micro",
        "regions": ["us-east-1", "eu-north-1", "ap-southeast-2", "us-west-1", "ap-northeast-1"]
    }
}
```
The first block (`key`) contains information regarding your SSH key:
```json
"key": {
    "name": "aws",
    "path": "/absolute/key/path"
},
```
Enter the name of your SSH key; this is the name you specified in the AWS web console in step 2. Also, enter the absolute path of your SSH private key (using a relative path won't work). 


The second block (`ports`) specifies the TCP ports to use:
```json
"port": 5000,
```
Narwhal requires a number of TCP ports, depening on the number of workers per node, Each primary requires 2 ports (one to receive messages from other primaties and one to receive messages from its workers), and each worker requires 3 ports (one to receive client transactions, one to receive messages from its primary, and one to receive messages from other workers). Note that the script will open a large port range (5000-9000) to the WAN on all your AWS instances. 

The third block (`repo`) contains the information regarding the repository's name, the URL of the repo, and the branch containing the code to deploy: 
```json
"repo": {
    "name": "narwhal",
    "url": "https://github.com/asonnino/narwhal.git",
    "branch": "master"
},
```
Remember to update the `url` field to the name of your repo. Modifying the branch name is particularly useful when testing new functionalities without having to checkout the code locally. 

The the last block (`instances`) specifies the [AWS instance type](https://aws.amazon.com/ec2/instance-types) and the [AWS regions](https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/using-regions-availability-zones.html#concepts-available-regions) to use:
```json
"instances": {
    "type": "t2.micro",
    "regions": ["us-east-1", "eu-north-1", "ap-southeast-2", "us-west-1", "ap-northeast-1"]
}
```
The instance type selects the hardware on which to deploy the testbed. For example, `t2.micro` instances come with 1 vCPU (1 physical core), and 1 GB of RAM. The python scripts will configure each instance with 300 GB of SSD hard drive. The `regions` field specifies the data centers to use. If you require more nodes than data centers, the python scripts will distribute the nodes as equally as possible amongst the data centers. All machines run a fresh install of Ubuntu Server 20.04.

### Step 4. Create a testbed
The AWS instances are orchestrated with [Fabric](http://www.fabfile.org) from the file [fabfile.py](https://github.com/akhil-sb/delphi-rs/blob/master/benchmark/fabfile.py) (located in [narwhal/benchmarks](https://github.com/akhil-sb/delphi-rs/blob/master/benchmark)); you can list all possible commands as follows:
```
$ cd delphi-rs/benchmark
$ fab --list
```
The command `fab create` creates new AWS instances; open [fabfile.py](https://github.com/asonnino/narwhal/blob/master/benchmark/fabfile.py) and locate the `create` task:
```python
@task
def create(ctx, nodes=2):
    ...
```
The parameter `nodes` determines how many instances to create in *each* AWS region. That is, if you specified 5 AWS regions as in the example of step 3, setting `nodes=2` will creates a total of 10 machines:
```
$ fab create

Creating 10 instances |██████████████████████████████| 100.0% 
Waiting for all instances to boot...
Successfully created 10 new instances
```
You can then clone the repo and install rust on the remote instances with `fab install`:
```
$ fab install

Installing rust and cloning the repo...
Initialized testbed of 10 nodes
```
This may take a long time as the command will first update all instances.
The commands `fab stop` and `fab start` respectively stop and start the testbed without destroying it (it is good practice to stop the testbed when not in use as AWS can be quite expensive); and `fab destroy` terminates all instances and destroys the testbed. Note that, depending on the instance types, AWS instances may take up to several minutes to fully start or stop. The command `fab info` displays a nice summary of all available machines and information to manually connect to them (for debug).

### Step 5. Run a benchmark
After setting up the testbed, running a benchmark on AWS is similar to running it locally (see [Run Local Benchmarks](https://github.com/asonnino/narwhal/tree/master/benchmark#local-benchmarks)). Locate the task `remote` in [fabfile.py](https://github.com/asonnino/narwhal/blob/master/benchmark/fabfile.py):
```python
@task
def remote(ctx):
    ...
```
Run the benchmark with the following command. 
```
$ fab remote
```
This command first updates all machines with the latest commit of the GitHub repo and branch specified in your file [settings.json](https://github.com/asonnino/narwhal/blob/master/benchmark/settings.json) (step 3); this ensures that benchmarks are always run with the latest version of the code. It then generates and uploads the configuration files to each machine, and runs the benchmarks with the specified parameters. The input parameters for Delphi can be set in the `_config` function in the (remote.py)[https://github.com/akhilsb/delphi-rs/benckmark/benchmark/remote.py] file in the `benchmark` folder. 

### Step 6: Download logs
The following command downloads the log file from the `syncer` titled `syncer.log`. 
```
$ fab logs
```
The `syncer.log` file contains the details about the latency of the protocol and the outputs of the nodes. Note that this log file needs to be downloaded only after allowing the protocol sufficient time to terminate (Ideally within 5 minutes). If anything goes wrong during a benchmark, you can always stop it by running `fab kill`.

## Running FIN
The FIN protocol requires the presence of a file with the name `tkeys.tar.gz`, which is a compressed file containing the BLS public key as `pub`, partial secret key shares as `sec0,...,sec{n-1}`, and corresponding public keys as `pub0,...,pubn-1`. This repository contains these keys for values of `n=16,64,112,160`. 

# Reproducing results in the paper
We ran Delphi at configuration of $\epsilon=2, \delta=20, \Delta = 2000$, and $\rho_0 =2$ in the Bitcoin usecase at $n=16,64,112,160$ nodes in a geo-distributed testbed of `t2.micro` nodes spread across 8 regions:  N. Virginia, Ohio, N. California, Oregon, Canada, Ireland, Singapore, and Tokyo. We also ran Delphi at a configuration of \epsilon=2, \delta=180, \Delta = 2000,\rho_0=2$ to demonstrate the performance at a high $\delta$. We ran FIN with the same configuration. However, FIN's runtime is independent of the inputs and input parameters. We ran Abraham et al. with $\epsilon=2, \delta=2, \Delta = 20$. Notice that $\delta$ in Delphi is different than Abraham et al. In Abraham et al., $\Delta$ is the real difference between honest inputs (It is the maximum difference in Delphi). 