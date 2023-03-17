use fuels::prelude::*;

#[tokio::test]
async fn main_function_generic_arguments() -> Result<()> {
    abigen!(Script(
        name = "MyScript",
        abi = "packages/fuels/tests/script_types/generic_types/out/debug/generic_types-abi.json"
    ));
    let wallet = launch_provider_and_get_wallet().await;
    let bin_path = "../fuels/tests/script_types/generic_types/out/debug/generic_types.bin";
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
        abi = "packages/fuels/tests/script_types/option_result/out/debug\
        /option_result-abi.json"
    ));
    let wallet = launch_provider_and_get_wallet().await;
    let bin_path = "../fuels/tests/script_types/option_result/out/debug/option_result.bin";
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
        abi = "packages/fuels/tests/script_types/tuple/out/debug/tuple-abi.json"
    ));
    let wallet = launch_provider_and_get_wallet().await;
    let bin_path = "../fuels/tests/script_types/tuple/out/debug/tuple.bin";
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
        abi = "packages/fuels/tests/script_types/vectors_script/out/debug/vectors_script-abi.json"
    ));
    let wallet = launch_provider_and_get_wallet().await;
    let bin_path = "../fuels/tests/script_types/vectors_script/out/debug/vectors_script.bin";
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
        abi = "packages/fuels/tests/script_types/raw_slice_script/out/debug/raw_slice_script-abi.json",
    ));

    let wallet = launch_provider_and_get_wallet().await;
    let bin_path = "../fuels/tests/script_types/raw_slice_script/out/debug/raw_slice_script.bin";
    let instance = BimBamScript::new(wallet.clone(), bin_path);

    for length in 0..=10 {
        let response = instance.main(length).call().await?;
        assert_eq!(response.value, (0..length).collect::<Vec<_>>());
    }
    Ok(())
}
