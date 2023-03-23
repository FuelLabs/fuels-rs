use fuels::prelude::*;

#[tokio::test]
async fn main_function_generic_arguments() -> Result<()> {
    abigen!(Script(
        name = "MyScript",
        abi =
            "packages/fuels/tests/types/scripts/script_generics/out/debug/script_generics-abi.json"
    ));
    let wallet = launch_provider_and_get_wallet().await;
    let bin_path = "../fuels/tests/types/scripts/script_generics/out/debug/script_generics.bin";
    let instance = MyScript::new(wallet, bin_path);

    let bim = GenericBimbam { val: 90 };
    let bam_comp = GenericBimbam { val: 4342 };
    let bam = GenericSnack {
        twix: bam_comp,
        mars: 1000,
    };
    let result = instance.main(bim.clone(), bam.clone()).call().await?;
    let expected = (
        GenericSnack {
            twix: GenericBimbam {
                val: bam.mars as u64,
            },
            mars: 2 * bim.val as u32,
        },
        GenericBimbam { val: 255_u8 },
    );
    assert_eq!(result.value, expected);
    Ok(())
}

#[tokio::test]
async fn main_function_option_result() -> Result<()> {
    abigen!(Script(
        name = "MyScript",
        abi =
            "packages/fuels/tests/types/scripts/options_results/out/debug/options_results-abi.json"
    ));
    let wallet = launch_provider_and_get_wallet().await;
    let bin_path = "../fuels/tests/types/scripts/options_results/out/debug/options_results.bin";
    let instance = MyScript::new(wallet, bin_path);

    let result = instance.main(Some(42), None).call().await?;
    assert_eq!(result.value, Ok(Some(true)));
    let result = instance.main(Some(987), None).call().await?;
    assert_eq!(result.value, Ok(None));
    let expected_error = Err(TestError::ZimZam("error".try_into().unwrap()));
    let result = instance.main(None, Some(987)).call().await?;
    assert_eq!(result.value, expected_error);
    Ok(())
}

#[tokio::test]
async fn main_function_tuple_types() -> Result<()> {
    abigen!(Script(
        name = "MyScript",
        abi = "packages/fuels/tests/types/scripts/script_tuples/out/debug/script_tuples-abi.json"
    ));
    let wallet = launch_provider_and_get_wallet().await;
    let bin_path = "../fuels/tests/types/scripts/script_tuples/out/debug/script_tuples.bin";
    let instance = MyScript::new(wallet, bin_path);

    let bim = Bim { bim: 90 };
    let bam = Bam {
        bam: "itest".try_into()?,
    };
    let boum = Boum { boum: true };
    let result = instance
        .main(
            (bim, bam, boum),
            Bam {
                bam: "secod".try_into()?,
            },
        )
        .call()
        .await?;
    let expected = (
        (
            Boum { boum: true },
            Bim { bim: 193817 },
            Bam {
                bam: "hello".try_into()?,
            },
        ),
        42242,
    );
    assert_eq!(result.value, expected);

    Ok(())
}

#[tokio::test]
async fn main_function_vector_arguments() -> Result<()> {
    abigen!(Script(
        name = "MyScript",
        abi = "packages/fuels/tests/types/scripts/script_vectors/out/debug/script_vectors-abi.json"
    ));
    let wallet = launch_provider_and_get_wallet().await;
    let bin_path = "../fuels/tests/types/scripts/script_vectors/out/debug/script_vectors.bin";
    let instance = MyScript::new(wallet, bin_path);

    let u32_vec = vec![0, 1, 2];
    let vec_in_vec = vec![vec![0, 1, 2], vec![0, 1, 2]];
    let struct_in_vec = vec![SomeStruct { a: 0 }, SomeStruct { a: 1 }];
    let vec_in_struct = SomeStruct { a: vec![0, 1, 2] };
    let array_in_vec = vec![[0u64, 1u64], [0u64, 1u64]];
    let vec_in_array = [vec![0, 1, 2], vec![0, 1, 2]];
    let vec_in_enum = SomeEnum::a(vec![0, 1, 2]);
    let enum_in_vec = vec![SomeEnum::a(0), SomeEnum::a(1)];

    let tuple_in_vec = vec![(0, 0), (1, 1)];
    let vec_in_tuple = (vec![0, 1, 2], vec![0, 1, 2]);
    let vec_in_a_vec_in_a_struct_in_a_vec = vec![
        SomeStruct {
            a: vec![vec![0, 1, 2], vec![3, 4, 5]],
        },
        SomeStruct {
            a: vec![vec![6, 7, 8], vec![9, 10, 11]],
        },
    ];

    let result = instance
        .main(
            u32_vec,
            vec_in_vec,
            struct_in_vec,
            vec_in_struct,
            array_in_vec,
            vec_in_array,
            vec_in_enum,
            enum_in_vec,
            tuple_in_vec,
            vec_in_tuple,
            vec_in_a_vec_in_a_struct_in_a_vec,
        )
        .call()
        .await?;

    assert!(result.value);

    Ok(())
}

#[tokio::test]
async fn test_script_raw_slice() -> Result<()> {
    abigen!(Script(
        name = "BimBamScript",
        abi = "packages/fuels/tests/types/scripts/script_raw_slice/out/debug/script_raw_slice-abi.json",
    ));

    let wallet = launch_provider_and_get_wallet().await;
    let bin_path = "../fuels/tests/types/scripts/script_raw_slice/out/debug/script_raw_slice.bin";
    let instance = BimBamScript::new(wallet.clone(), bin_path);

    let raw_slice = RawSlice(vec![40, 41, 42]);
    let wrapper = Wrapper {
        inner: vec![raw_slice.clone(), raw_slice.clone()],
        inner_enum: SomeEnum::Second(raw_slice),
    };

    let rtn = instance.main(10, wrapper).call().await?.value;
    assert_eq!(rtn, RawSlice(vec![1, 2, 3]));

    Ok(())
}

#[tokio::test]
async fn main_function_bytes_arguments() -> Result<()> {
    abigen!(Script(
        name = "MyScript",
        abi = "packages/fuels/tests/types/scripts/script_bytes/out/debug/script_bytes-abi.json"
    ));
    let wallet = launch_provider_and_get_wallet().await;
    let bin_path = "../fuels/tests/types/scripts/script_bytes/out/debug/script_bytes.bin";
    let instance = MyScript::new(wallet, bin_path);

    let bytes = Bytes(vec![40, 41, 42]);
    let wrapper = Wrapper {
        inner: vec![bytes.clone(), bytes.clone()],
        inner_enum: SomeEnum::Second(bytes),
    };

    instance.main(10, wrapper).call().await?;

    Ok(())
}
