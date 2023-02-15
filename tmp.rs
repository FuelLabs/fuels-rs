impl From<MyContractConfigurable> for ::fuels::programs::contract::ReplaceConfigurable {
    fn from(configurable: MyContractConfigurable) -> Self {
        :: fuels :: programs :: contract :: ReplaceConfigurable        {            configurables : vec!            [(120u64, :: fuels :: core :: abi_encoder :: ABIEncoder ::            encode(&            [:: fuels :: types :: SizedAsciiString < 5usize > ::            into_token(configurable.STR),]).expect("Cannot encode configurable data").resolve(0)),
     (128u64, :: fuels :: core :: abi_encoder :: ABIEncoder ::            encode(&            [[u8 ; 3usize] ::            into_token(configurable.ARR),]).expect("Cannot encode configurable data").resolve(0)),            (152u64, ::
fuels :: core :: abi_encoder :: ABIEncoder ::            encode(&
[:: fuels :: types :: SizedAsciiString < 4usize > ::            into_token(configurable.STR2),]).expect("Cannot encode configurable data").resolve(0))],
    }
    }
}
