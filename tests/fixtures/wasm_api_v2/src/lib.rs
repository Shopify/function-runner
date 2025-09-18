use anyhow::{anyhow, Result};

#[export_name = "_start"]
fn start() {
    main().unwrap()
}

fn main() -> Result<()> {
    let mut ctx = shopify_function_wasm_api::Context::new();
    let input = ctx.input_get()?;
    let str = input
        .get_obj_prop("hello")
        .as_string()
        .ok_or_else(|| anyhow!("Should be string"))?;
    ctx.write_object(
        |ctx| {
            ctx.write_utf8_str("bye")?;
            ctx.write_utf8_str(&str)?;
            Ok(())
        },
        1,
    )?;
    // Test log wrap-around by writing 1011 entries (capacity is 1001).
    ctx.log(&"a".repeat(1001));
    ctx.log(&"b".repeat(10));
    Ok(())
}
