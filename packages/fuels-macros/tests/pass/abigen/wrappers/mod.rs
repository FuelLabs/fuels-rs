macro_rules! definition_site_abi {
    () => {
        fuels_macros::wasm_abigen!(Contract(name = "Defined", abi = "../nested/abi.json"));
    };
}

macro_rules! forwarded_abi {
    ($abi:literal) => {
        fuels_macros::wasm_abigen!(Contract(name = "Forwarded", abi = $abi));
    };
}
