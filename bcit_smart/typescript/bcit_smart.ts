// @ts-ignore
import { config } from "./bcit_smart_config.js"; // @ts-ignore
import * as util from "../odin_server/ui_util.js"; // @ts-ignore
import * as ui from "../odin_server/ui.js"; // @ts-ignore
import * as ws from "../odin_server/ws.js"; // @ts-ignore
import * as odinCesium from "../odin_cesium/odin_cesium.js"; // @ts-ignore

declare const Cesium: typeof import("cesium");

const MODULE_PATH = util.asset_path(import.meta.url);

const LINE_TYPE = "testline";
const POINT_TYPE = "testPoint";
const LINE_SETTINGS = "powerlineSettings";
const LINE_DETAILS = "lineDetails";

ws.addWsHandler( MODULE_PATH, handleWsMessages);

//--- display params we can change from config file can be extracted here


createIcon();
createSettingsWindow();
createDetailsWindow();
initPowerLineDetailsView();

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
    return ui.Icon("./asset/odin_cesium/globe.svg", (e)=> ui.toggleWindow(e,LINE_SETTINGS));
}

function createSettingsWindow() {
    return ui.Window("Test Map-Lines", LINE_SETTINGS, "./asset/bcit_smart/button_svg.svg")(
        ui.LayerPanel(LINE_SETTINGS, toggleShowLines),
        ui.Button("Send message back to server", sendHello)
    );
}

function sendHello() {
    console.log("Test of sending message back to server");
    ws.sendWsMessage("bcit_smart/bcit_smart", "string", "hello");
}

function createDetailsWindow() {
    return ui.Window("Line Details", LINE_DETAILS, "./asset/bcit_smart/button_svg.svg")(
        // ui.List("testlines.selectedLine", 4, selectGoesrDataSet),
        ui.Panel("data sets", true)(
            ui.CheckBox("show lines", toggleShowLines, "lines"),
            ui.List("powerlines.selectedPowerline", 3, () => console.log("When is this called?")),
        )
    );
}

function toggleShowLines(event) {
    let cb = ui.getCheckBox(event.target);
    console.log(event.target);
    if (cb) {
        toggleTestLines( ui.isCheckBoxSelected(cb));
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
                _type : LINE_TYPE
            } as any);
    
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
        console.log("Window should open");;
        console.log("Selection", sel);
        ui.showWindow(LINE_DETAILS);
        ui.setWindowLocation(ui.getWindow(LINE_DETAILS), 200, 200);
        ui.setListItems("powerlines.selectedPowerline", [sel])
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
        case "powerlines": handlePowerLineDataSet(msg); break;
    }
}

/**
 * Need to change the datashape to match what Cesium expects.
 * Consider doing this transformation on the server later.
 */
function convertServerPowerlineToCesiumLines (powerlines) {
    return powerlines.map(line => ({
        coordinates: line.positions.flatMap(({lat_deg, lon_deg}) => [lat_deg, lon_deg]),
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
function handlePowerLineDataSet({powerlines}) {
    console.log("Received a message about the power lines");
    console.log(powerlines);
    console.log("Going add the lines to the map");
    
    if (lineDataSource === null) {
        lineDataSource = new Cesium.CustomDataSource(LINE_TYPE);
    }

    // Think about how to handle updates as in replacing data // updating already received data etc..
    // For now just going to replace everything on update
    lineDataSource.entities.removeAll();
    
    // Iterate over each line and create an entity for it
    convertServerPowerlineToCesiumLines(powerlines).forEach(line => {
        console.log("adding line", line);
        const lineEntity = new Cesium.Entity({
            polyline: {
                positions: Cesium.Cartesian3.fromDegreesArray(line.coordinates),
                width: line.width || 2, // Default width if not specified
                material: line.color || Cesium.Color.RED // Default color if not specified
            },
            _type : LINE_TYPE,
            powId: line.powId,
            time: line.time
        } as any);

        // Add the line entity to the data source
        lineDataSource.entities.add(lineEntity);
    });

    // Add the data source to the map
    odinCesium.requestRender();
}

function initPowerLineDetailsView() {
    const view = ui.getList("powerlines.selectedPowerline");
    if (view) {
        ui.setListItemDisplayColumns(view, ["fit", "header"], [
            { name: "powId", tip: "Id of line", width: "3rem", attrs: [], map: e => e.powId },
            { name: "time", tip: "last report", width: "8rem", attrs: ["fixed", "alignRight"], map: e => e.time }
        ]);
    }
    return view;
}