# Delphi: Asynchronous Approximate Agreement for Distributed Oracles
This repository contains a Rust implementation of the following distributed oracle agreement protocols. 

1. Delphi AAA protocol
2. FIN ACS protocol [1]
3. Abraham et al. AAA protocol [2]

The repository uses the libchatter networking library available [here](https://github.com/libdist-rs/libchatter-rs). This code has been written as a research prototype and has not been vetted for security. Therefore, this repository can contain serious security vulnerabilities. Please use at your own risk. 

## Dataset
The repository also contains a dataset containing values of prominent cryptocurrencies polled from 12 cryptocurrency exchanges. Details are available in the `dataset` folder. 

# Quick Start
The repository uses the `Cargo` build tool. The compatibility between dependencies has been tested for Rust version `1.63`. 

Build the repository using the following command. 
```
$ cargo build --release
```
Next, generate configuration files for nodes in the system using the following command. 
```
$ ./target/release/genconfig --base_port 8500 --client_base_port 7000 --client_run_port 9000 --NumNodes 4 --blocksize 100 --delay 100 --target testdata/hyb_4/ --local true
```
After generating the configuration files, run the script `appxcon-test.sh` in the scripts folder with the following command line arguments. This command starts Delphi with four nodes.
```
$ ./scripts/appxcon-test.sh {epsilon} {rho} {Delta} testdata/hyb_4/syncer
```
Substitute desired values of $\epsilon,\rho_0,\Delta$. Example values include $\epsilon=1,\rho_0=10,\Delta=100000$. The script randomly assigns input values $v_i$ to each node. This logic can be changed to make nodes start with custom input values. 

The outputs are logged into the `syncer.log` file in logs directory. The outputs of each node are printed in a JSON format, along with the amount of time the node took to terminate the protocol. 

Running the FIN ACS protocol requires additional configuration. FIN uses BLS threshold signatures to generate common coins necessary for proposal election and Binary Byzantine Agreement. This setup includes a master public key in the `pub` file, $n$ partial secret keys (one for each node) as `sec0,...,sec3` files, and the $n$ partial public keys as `pub0,...,pub3` files. We utilized the `crypto_blstrs` library in the [apss](https://github.com/ISTA-SPiDerS/apss) repository to generate these keys. 

## Running in AWS
We utilize the code in the [Narwhal](https://github.com/MystenLabs/sui/tree/main/narwhal/benchmark) repository to execute code in AWS. This repository uses `fabric` to spawn AWS instances, install Rust, and build the repository on individual machines. Please refer to the `benchmark` directory for more instructions. 

# System architecture
Each node runs as an independent process, which communicates with other nodes through sockets. Apart from the $n$ nodes running the protocol, the system also spawns a process called `syncer`. The `syncer` is responsible for measuring latency of completion. It reliably measures the system's latency by issuing `START` and `STOP` commands to all nodes. The nodes begin executing the protocol only after the `syncer` verifies that all nodes are online, and issues the `START` command by sending a message to all nodes. Further, the nodes send a `TERMINATED` message to the `syncer` once they terminate the protocol. The `syncer` records both start and termination times of all processes, which allows it to accurately measure the latency of each protocol. 