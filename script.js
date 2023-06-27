const _ = require('lodash');
const { v4: uuidv4 } = require('uuid');
const fs = require('fs');

// generate 250 locations and 10 fulfillment_groups
function generateJson() {
    let locationRule = {};
    let fulfillmentGroups = [];
    let locations = [];

    let location_ids = Array.from({length: 250}, () => Math.floor(Math.random() * Math.pow(2, 64)));
    let shuffledLocations = _.shuffle(location_ids);
    let priority = 0;

    for (let i = 0; i < 10; i++) {
        let groups = [];
        for (let j = 0; j < 10; j+=2) {
            let index = i*10 + j
            let locations = shuffledLocations.slice(index, index+2).map(location => `gid://shopify/Location/${location}`);
            groups.push({
                "name": "Location Group",
                "locations": locations,
                "priority": priority++
            });
        }
        locationRule[`functionConfiguration${i+1}`] = {
            "value": JSON.stringify({ "groups": groups })
        };
    }

    let fulfillment_group_ids = Array.from({length: 10}, () => uuidv4());

    for (let id of fulfillment_group_ids) {
        let inventoryLocations = [];
        let numLocations = Math.floor(Math.random() * location_ids.length) + 1;
        for (let i = 0; i < numLocations; i++) {
            let location = location_ids[Math.floor(Math.random() * location_ids.length)];
            inventoryLocations.push({
                "location": { "id": `gid://shopify/Location/${location}` }
            });
        }
        fulfillmentGroups.push({
            "id": `gid://shopify/FulfillmentGroup/${id}`,
            "inventoryLocations": inventoryLocations
        });
    }

    location_ids.forEach(id => {
        locations.push({
            "id": `gid://shopify/Location/${id}`
        });
    });


    return {
        "locationRule": locationRule,
        "fulfillmentGroups": fulfillmentGroups,
        "locations": locations
    };
}

let data = generateJson();

// fs.writeFile('input-2500.json', JSON.stringify(data, null, 2), (err) => {
//     if (err) throw err;
//     console.log('Data written to file');
// });
fs.writeFile('input-2500-min.json', JSON.stringify(data), (err) => {
    if (err) throw err;
    console.log('Data written to file');
});
