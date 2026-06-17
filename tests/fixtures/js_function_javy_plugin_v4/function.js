const inputObj = ShopifyFunction.readInput();
console.error("this is an error message");
const outputObj = { hello: inputObj.hello + " output" };
ShopifyFunction.writeOutput(outputObj);
