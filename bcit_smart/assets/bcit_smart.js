/* eslint-disable @typescript-eslint/ban-ts-comment */
// @ts-ignore
import { config } from "./bcit_smart_config.js"; // @ts-ignore
import * as util from "../odin_server/ui_util.js"; // @ts-ignore
import * as ui from "../odin_server/ui.js"; // @ts-ignore
import * as ws from "../odin_server/ws.js"; // @ts-ignore
import * as odinCesium from "../odin_cesium/odin_cesium.js"; // @ts-ignore
const MODULE_PATH = util.asset_path(import.meta.url);
const LINE_TYPE = "testline";
const POINT_TYPE = "testPoint";
const LINE_SETTINGS = "powerlineSettings";
const LINE_DETAILS = "lineDetails";
ws.addWsHandler(MODULE_PATH, handleWsMessages);
//--- display params we can change from config file can be extracted here
let selectedPowerline = [];
createIcon();
createSettingsWindow();
createDetailsWindow();
initPowerLineDetailsView();
let pointDataSource = null;
// initOasisPoints();
// function initOasisPoints() {
//     const point = [ -122.9994, 49.2497];
//     if (!pointDataSource) {
//         pointDataSource = new Cesium.CustomDataSource("oasisPoints");
//         odinCesium.addDataSource(pointDataSource);
//     }
//     const pointEntity = new Cesium.Entity({
//         position: Cesium.Cartesian3.fromDegrees(point[0], point[1]),
//         point: {
//             pixelSize: 10,
//             color: Cesium.Color.RED,
//         },
//         description: "Oasis EV Chargers", // Tooltip text for the point
//         name: "Oasis EV Chargers", // Name of the entity
//         _type: POINT_TYPE,
//         label: {
//             text: "Oasis Point",
//             font: config.font,
//             fillColor: config.outlineColor,
//             showBackground: true,
//             backgroundColor: config.labelBackground,
//             //heightReference: Cesium.HeightReference.CLAMP_TO_GROUND,
//             horizontalOrigin: Cesium.HorizontalOrigin.LEFT,
//             verticalOrigin: Cesium.VerticalOrigin.TOP,
//             pixelOffset: new Cesium.Cartesian2( 5, 5),
//             scaleByDistance: new Cesium.NearFarScalar(
//                 500.0, 1.0, // Full visibility at 100 meters
//                 2000.0, 0.4 // Half visibility at 1000 meters
//             )
//         }
//     } as any);
//     const point2 = [-122.9985, 49.2493];
//     const pointEntity2  = new Cesium.Entity({
//         position: Cesium.Cartesian3.fromDegrees(point2[0], point2[1]),
//         point: {
//             pixelSize: 10,
//             color: Cesium.Color.RED,
//         },
//         description: "Oasis Point Example", // Tooltip text for the point
//         name: "Oasis Point", // Name of the entity
//         _type: POINT_TYPE,
//         label: {
//             text: "Oasis Battery Bank",
//             font: config.font,
//             fillColor: config.outlineColor,
//             showBackground: true,
//             backgroundColor: config.labelBackground,
//             //heightReference: Cesium.HeightReference.CLAMP_TO_GROUND,
//             horizontalOrigin: Cesium.HorizontalOrigin.LEFT,
//             verticalOrigin: Cesium.VerticalOrigin.TOP,
//             pixelOffset: new Cesium.Cartesian2( 5, 5),
//             scaleByDistance: new Cesium.NearFarScalar(
//                 500.0, 1.0, // Full visibility at 100 meters
//                 2000.0, 0.4 // Half visibility at 1000 meters
//             )
//         }
//     } as any);
//     // Add the point entity to the data source
//     pointDataSource.entities.add(pointEntity);
//     pointDataSource.entities.add(pointEntity2);
//     // Request a render update (if necessary)
//     odinCesium.requestRender();
// }
// Handles clicking on a powerline
odinCesium.setEntitySelectionHandler(powerlineSelection);
odinCesium.initLayerPanel(LINE_SETTINGS, config, toggleTestLines);
odinCesium.initLayerPanel(LINE_DETAILS, config, () => null);
const lines = [
    { coordinates: [-110.0, 37.0, -115.1, 38.0, -105.1, 39.0], color: Cesium.Color.BLUE, width: 3 },
    { coordinates: [-110.1, 38.0, -115.2, 39.0], color: Cesium.Color.GREEN, width: 2 }
];
let lineDataSource = null;
if (config.layer.show) {
    console.log("should show lines on load");
    initLines();
}
console.log("ui_bcit_smart initialized");
function createIcon() {
    return ui.Icon("./asset/odin_cesium/globe.svg", (e) => ui.toggleWindow(e, LINE_SETTINGS));
}
function createSettingsWindow() {
    return ui.Window("Test Map-Lines", LINE_SETTINGS, "./asset/bcit_smart/button_svg.svg")(ui.LayerPanel(LINE_SETTINGS, toggleShowLines), ui.Button("Send message back to server", sendHello));
}
function sendHello() {
    console.log("Test of sending message back to server");
    ws.sendWsMessage("bcit_smart/bcit_smart", "string", "hello");
}
function createDetailsWindow() {
    return ui.Window("Line Details", LINE_DETAILS, "./asset/bcit_smart/button_svg.svg")(
    // ui.List("testlines.selectedLine", 4, selectGoesrDataSet),
    ui.Panel("data sets", true)(ui.CheckBox("show lines", toggleShowLines, "lines"), ui.List("powerlines.selectedPowerline", 3, () => console.log("When is this called?"))));
}
function toggleShowLines(event) {
    let cb = ui.getCheckBox(event.target);
    console.log(event.target);
    if (cb) {
        toggleTestLines(ui.isCheckBoxSelected(cb));
    }
}
function initLines() {
    console.log("Call to init Lines lineDataSource: " + lineDataSource);
    if (lineDataSource === null) {
        console.log("Creating Test Lines onto map");
        // Create a new data source for lines
        lineDataSource = new Cesium.CustomDataSource(LINE_TYPE);
        // Iterate over each line and create an entity for it
        lines.forEach(line => {
            const lineEntity = new Cesium.Entity({
                polyline: {
                    positions: Cesium.Cartesian3.fromDegreesArray(line.coordinates),
                    width: line.width || 2, // Default width if not specified
                    material: line.color || Cesium.Color.RED // Default color if not specified
                },
                _type: LINE_TYPE
            });
            // Add the line entity to the data source
            lineDataSource.entities.add(lineEntity);
        });
        // Add the data source to the map
        odinCesium.addDataSource(lineDataSource);
        odinCesium.requestRender();
    }
}
function toggleTestLines(showLines) {
    console.log("Toggle Test Lines" + showLines);
    if (lineDataSource === null) {
        initLines();
    }
    lineDataSource.show = showLines ?? true;
    odinCesium.requestRender();
}
function clearEntities() {
    if (lineDataSource) {
        lineDataSource.entities.removeAll();
    }
}
function powerlineSelection() {
    let sel = odinCesium.getSelectedEntity();
    if (sel && sel._type && sel._type == LINE_TYPE) {
        console.log("Window should open");
        ;
        console.log("Selection", sel);
        ui.showWindow(LINE_DETAILS);
        ui.setWindowLocation(ui.getWindow(LINE_DETAILS), 200, 200);
        ui.setListItems("powerlines.selectedPowerline", [sel]);
    }
    console.log("selected entity: ");
    console.log(sel);
}
//--- data messages
function handleWsMessages(msgType, msg) {
    console.log("GOT A WS MESSAGE");
    console.log(msgType);
    console.log(msg);
    switch (msgType) {
        case "powerlines":
            handlePowerLineDataSet(msg);
            break;
    }
}
/**
 * Need to change the datashape to match what Cesium expects.
 * Consider doing this transformation on the server later.
 */
function convertServerPowerlineToCesiumLines(powerlines) {
    return powerlines.map(line => ({
        coordinates: line.positions.flatMap(({ lat_deg, lon_deg }) => [lat_deg, lon_deg]),
        ...line
    }));
}
/** Type is:
*   {
*       powerlines: {
*           powId: Number,
*           positions: {lat_deg: Number, lon_deg: Number}[],
*           time: String
*       }[]
*   }
*/
function handlePowerLineDataSet({ powerlines }) {
    console.log("Received a message about the power lines");
    console.log(powerlines);
    console.log("Going add the lines to the map");
    if (lineDataSource === null) {
        lineDataSource = new Cesium.CustomDataSource(LINE_TYPE);
    }
    // Think about how to handle updates as in replacing data // updating already received data etc..
    // For now just going to replace everything on update
    lineDataSource.entities.removeAll();
    // const test_line_data = [
    //     [
    //         [-123.10486705, 49.266949997],
    //         [-123.103687403, 49.266935119],
    //         [-123.103635106, 49.266934209],
    //     ],
    //     [
    //         [-123.103687659, 49.266928374],
    //         [-123.103937444, 49.2668337]
    //     ],
    //     [
    //         [-123.103687659, 49.266928374],
    //         [-123.103691751, 49.266820465]
    //     ],
    //     [
    //         [-123.103687659, 49.266928374],
    //         [-123.103635106, 49.266934209]
    //     ],
    //     [
    //         [-123.103687659, 49.266928374],
    //         [-123.103450979, 49.266820238]
    //     ],
    //     [
    //         [-123.103635106, 49.266934209],
    //         [-123.103169354, 49.266985916]
    //     ],
    //     [
    //         [-123.103635106, 49.266934209],
    //         [-123.1029626, 49.266922508]
    //     ]
    // ]
    // test_line_data.forEach((line, index) => {
    //     const lineEntity = new Cesium.Entity({
    //         polyline: {
    //             positions: Cesium.Cartesian3.fromDegreesArray(line.flat()),
    //             width: 2, // Default width if not specified
    //             material: Cesium.Color.RED // Default color if not specified
    //         },
    //         _type : LINE_TYPE,
    //         powId: index,
    //         time: "2021-08-12 12:00:00"
    //     } as any);
    //     // Add the line entity to the data source
    //     lineDataSource.entities.add(lineEntity);
    // });
    // Iterate over each line and create an entity for it
    convertServerPowerlineToCesiumLines(powerlines).forEach(line => {
        console.log("adding line", line);
        const lineEntity = new Cesium.Entity({
            polyline: {
                positions: Cesium.Cartesian3.fromDegreesArray(line.coordinates),
                width: line.width || 2, // Default width if not specified
                material: line.color || Cesium.Color.RED // Default color if not specified
            },
            _type: LINE_TYPE,
            powId: line.powId,
            time: line.time
        });
        // Add the line entity to the data source
        lineDataSource.entities.add(lineEntity);
    });
    // Add the data source to the map
    odinCesium.requestRender();
}
function initPowerLineDetailsView() {
    let view = ui.getList("powerlines.selectedPowerline");
    if (view) {
        ui.setListItemDisplayColumns(view, ["fit", "header"], [
            { name: "powId", tip: "Id of line", width: "3rem", attrs: [], map: e => e.powId },
            { name: "time", tip: "last report", width: "8rem", attrs: ["fixed", "alignRight"], map: e => e.time }
        ]);
    }
    return view;
}
