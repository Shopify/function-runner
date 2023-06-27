const fs = require('fs');

fs.readFile('input-2500-min.json', 'utf8', (err, jsonString) => {
    if (err) {
        console.log("Error reading file from disk:", err);
        return;
    }
    try {
        const data = JSON.parse(jsonString);
        const locationIdToIndex = {};
        data.locations.forEach((location, index) => {
            locationIdToIndex[location.id] = index;
        });

        const newFulfillmentGroups = data.fulfillmentGroups.map(group => {
            const inventoryLocationIndices = group.inventoryLocations.map(location => locationIdToIndex[location.location.id]);
            return {
                id: group.id,
                inventoryLocationIndices
            };
        });

        const newData = {
            locationRule: data.locationRule,
            fulfillmentGroups: newFulfillmentGroups,
            locations: data.locations
        };

        fs.writeFile('input-2500-indices-min.json', JSON.stringify(newData), err => {
            if (err) {
                console.log('Error writing file', err);
            } else {
                console.log('Successfully wrote file');
            }
        });
    } catch(err) {
        console.log('Error parsing JSON string:', err);
    }
});
