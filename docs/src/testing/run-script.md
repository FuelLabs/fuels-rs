# run_script 

`run_script` is helper function for testing simple Sway scripts and reducing boilerplate code related to setting up contracts and deployment.
It takes the path to the generated `.bin` file as argument.

You can use it this way:

````rust
#[tokio::test]
async fn test_logging_sway() {

    let path_to_bin = "tests/test_projects/logging/out/debug/logging.bin";
    let return_val = run_script(path_to_bin).await;

    let correct_hex =
        hex::decode("ef86afa9696cf0dc6385e2c407a6e159a1103cefb7e2ae0636fb33d3cb2a9e4a");

    assert_eq!(correct_hex.unwrap(), return_val[0].data().unwrap());

}
````
