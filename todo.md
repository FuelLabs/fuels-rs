[ ] Re-exporting types in a single place — users will only care about this single place to pull types.
[ ] Keep trait impl inside fuels-core, move specific types to fuels-types
[ ] We’re moving traits impl from lib.rs to new file name TBD
[ ] Check if a symbol can be imported through only one path
[ ] A package can only export symbols it owns
[x] Define the role of fuels-contract and rename it to what ever is appropriate
[x] Delete tools/fuels-abi-cli from the repo (It should probably live somewhere else in Fuels)

export some `fuel_types` in `fuels_types` check what we did use and export only that

graph_ql and fuel_tx and fuel_vm

move types from fuel-core to fuels_types and implement the parameterize thing there

