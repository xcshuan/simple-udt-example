use super::*;
use ckb_testtool::builtin::ALWAYS_SUCCESS;
use ckb_testtool::ckb_types::core::Capacity;
use ckb_testtool::ckb_types::core::TransactionBuilder;
use ckb_testtool::ckb_types::packed::{CellDep, CellInput, CellOutput};
use ckb_testtool::ckb_types::prelude::{Builder, Entity, Pack};
use ckb_testtool::context::random_hash;
const MAX_CYCLES: u64 = 100_0000;

fn assert_script_error(err: Error, err_code: i8) {
    let error_string = err.to_string();
    assert!(
        error_string.contains(format!("error code {} ", err_code).as_str()),
        "error_string: {}, expected_error_code: {}",
        error_string,
        err_code
    );
}

// errors
const ERROR_AMOUNT: i8 = 5;

fn build_test_context(
    inputs_token: Vec<u128>,
    outputs_token: Vec<u128>,
    is_owner_mode: bool,
) -> (Context, TransactionView) {
    // deploy simple-udt script
    let mut context = Context::default();

    let mut cell_deps = vec![];
    let mut inputs = vec![];
    let mut outputs = vec![];
    let mut outputs_data = vec![];

    let sudt_bin: Bytes = Loader::default().load_binary("simple-udt");
    let sudt_out_point = context.deploy_cell(sudt_bin);
    // deploy always_success script
    let always_success_out_point = context.deploy_cell(ALWAYS_SUCCESS.clone());

    // build lock script
    let lock_script = context
        .build_script(&always_success_out_point, Default::default())
        .expect("script");
    let lock_script_dep = CellDep::new_builder()
        .out_point(always_success_out_point)
        .build();
    cell_deps.push(lock_script_dep);

    // build sudt script
    let sudt_script_args: Bytes = if is_owner_mode {
        // use always_success script hash as owner's lock
        lock_script.calc_script_hash().as_bytes()
    } else {
        // use zero hash as owner's lock which implies we can never enter owner mode
        random_hash().as_bytes()
    };

    let sudt_script = context
        .build_script(&sudt_out_point, sudt_script_args)
        .expect("script");
    let sudt_script_dep = CellDep::new_builder().out_point(sudt_out_point).build();
    cell_deps.push(sudt_script_dep);

    // prepare inputs
    // assign 1000 Bytes to per input
    let input_ckb = Capacity::bytes(1000).unwrap().as_u64();
    inputs.extend(inputs_token.iter().map(|token| {
        let input_out_point = context.create_cell(
            CellOutput::new_builder()
                .capacity(input_ckb.pack())
                .lock(lock_script.clone())
                .type_(Some(sudt_script.clone()).pack())
                .build(),
            token.to_le_bytes().to_vec().into(),
        );
        let input = CellInput::new_builder()
            .previous_output(input_out_point)
            .build();
        input
    }));

    // prepare outputs
    let output_ckb = input_ckb * inputs_token.len() as u64 / outputs_token.len() as u64;
    outputs.extend(outputs_token.iter().map(|_token| {
        CellOutput::new_builder()
            .capacity(output_ckb.pack())
            .lock(lock_script.clone())
            .type_(Some(sudt_script.clone()).pack())
            .build()
    }));
    outputs_data.extend(
        outputs_token
            .iter()
            .map(|token| Bytes::from(token.to_le_bytes().to_vec())),
    );

    // build transaction
    let tx = TransactionBuilder::default()
        .cell_deps(cell_deps)
        .inputs(inputs)
        .outputs(outputs)
        .outputs_data(outputs_data.pack())
        .build();
    (context, tx)
}

#[test]
fn test_basic() {
    let (mut context, tx) = build_test_context(vec![1000], vec![400, 600], false);
    let tx = context.complete_tx(tx);

    // run
    let cycles = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("pass verification");
    println!("cycles: {}", cycles);
}

#[test]
fn test_destroy_udt() {
    let (mut context, tx) = build_test_context(vec![1000], vec![800, 100, 50], false);
    let tx = context.complete_tx(tx);

    // run
    let cycles = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("pass verification");
    println!("cycles: {}", cycles);
}

#[test]
fn test_create_sudt_without_owner_mode() {
    let (mut context, tx) = build_test_context(vec![1000], vec![1200], false);
    let tx = context.complete_tx(tx);

    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_AMOUNT);
}

#[test]
fn test_create_sudt_with_owner_mode() {
    let (mut context, tx) = build_test_context(vec![1000], vec![1200], true);
    let tx = context.complete_tx(tx);

    // run
    let cycles = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("pass verification");
    println!("cycles: {}", cycles);
}
