const fs = require('fs');

fs.readFile('input-2500-min.json', 'utf8', (err, jsonString) => {
    if (err) {
        console.log("File read failed:", err);
        return;
    }

    const data = JSON.parse(jsonString);

    const locationIdToShortId = {};
    data.locations.forEach((location, index) => {
        const shortId = location.id.split('/').pop();
        locationIdToShortId[location.id] = shortId;
        location.short_id = shortId;
    });

    Object.values(data.locationRule).forEach(rule => {
        const ruleValue = JSON.parse(rule.value);
        ruleValue.groups.forEach(group => {
            group.locations = group.locations.map(locationId => locationIdToShortId[locationId]);
        });
        rule.value = JSON.stringify(ruleValue);
    });

    data.fulfillmentGroups.forEach(group => {
        group.inventoryLocationShortIds = group.inventoryLocations.map(location => locationIdToShortId[location.location.id]);
        delete group.inventoryLocations;
    });

    fs.writeFile('input-2500-short-id-min.json', JSON.stringify(data), (err) => {
        if (err) throw err;
        console.log('Data written to file');
    });
});
