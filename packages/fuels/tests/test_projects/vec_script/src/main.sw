script;

use vec_lib::MyContract;
use std::vec::Vec;

fn main() {
	let mut a_vec = ~Vec::new();
	a_vec.push(100);
	a_vec.push(200);
	a_vec.push(300);

	let call_me = abi(MyContract, 0x53dce1040e5dc124cd5c80ffde7053b3402974fe315159a4fed27054b76af01f);
	call_me.is_the_correct_vec_given(a_vec);

}
