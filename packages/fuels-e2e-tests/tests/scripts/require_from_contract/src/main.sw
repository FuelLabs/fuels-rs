script;

use library::LibContract;

fn main(contract_id: ContractId) {
    let contract_instance = abi(LibContract, contract_id.into());
    contract_instance.require();
}
