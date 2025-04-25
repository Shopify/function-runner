// ../../node_modules/@shopify/shopify_function/run.ts
function run_default(userfunction) {
  if (!ShopifyFunction) {
    throw new Error(
      "ShopifyFunction is not defined. Please rebuild your function using the latest version of Shopify CLI."
    );
  }
  const input_obj = ShopifyFunction.readInput();
  const output_obj = userfunction(input_obj);
  ShopifyFunction.writeOutput(output_obj);
}

// src/run.js
var EMPTY_DISCOUNT = {
  discountApplicationStrategy: "FIRST" /* First */,
  discounts: []
};
function run(input) {
  const configuration = JSON.parse(
    input?.discountNode?.metafield?.value ?? "{}"
  );
  return EMPTY_DISCOUNT;
}

// <stdin>
function run2() {
  return run_default(run);
}
export {
  run2 as run
};
