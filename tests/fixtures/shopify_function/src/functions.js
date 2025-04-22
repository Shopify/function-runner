// var __defProp = Object.defineProperty;
// var __export = (target, all) => {
//   for (var name in all)
//     __defProp(target, name, { get: all[name], enumerable: true });
// };

// // extensions/volume-js/src/index.js
// var src_exports = {};
// __export(src_exports, {
//   default: () => src_default
// });
// var EMPTY_DISCOUNT = {
//   discountApplicationStrategy: "FIRST" /* First */,
//   discounts: []
// };
// var src_default = (input) => {
//   const configuration = JSON.parse(
//     input?.discountNode?.metafield?.value ?? "{}"
//   );
//   let cartLines = input.cart.lines;
//   if (cartLines.length == 0 || configuration.percentage == 0) {
//     return EMPTY_DISCOUNT;
//   }
//   let targets = [];
//   cartLines.forEach((line) => {
//     if (line.quantity >= configuration.quantity) {
//       targets.push({
//         productVariant: {
//           id: line.merchandise.id
//         }
//       });
//     }
//   });
//   if (targets.length == 0) {
//     return EMPTY_DISCOUNT;
//   }
//   return {
//     discountApplicationStrategy: "FIRST" /* First */,
//     discounts: [
//       {
//         value: {
//           percentage: {
//             value: configuration.percentage
//           }
//         },
//         targets
//       }
//     ]
//   };
// };

// // node_modules/@shopify/shopify_function/index.ts
// var input_data = ShopifyFunction.readInput();
// var output_obj = src_exports?.default(input_data);
// ShopifyFunction.writeOutput(output_obj);

console.log("hello");
console.log(ShopifyFunction.readInput());
ShopifyFunction.writeOutput(1);