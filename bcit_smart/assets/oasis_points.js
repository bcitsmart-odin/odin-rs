// These are just for VSCode's intellisense, comment them out when compiling or won't work correctly
// I could not figure out a good way to handle how the files change places when run
// @ts-ignore
// declare const util: typeof import("../../odin_server/assets/ui_util.js"); // @ts-ignore
// declare const ws: typeof import("../../odin_server/assets/ws.js"); // @ts-ignore
// declare const ui: typeof import("../../odin_server/assets/ui.js"); // @ts-ignore
// declare const odinCesium: typeof import("../../odin_cesium/assets/odin_cesium.js");
// @ts-ignore
import { config } from "./oasis_config.js"; // @ts-ignore
import * as util from "../odin_server/ui_util.js"; // @ts-ignore
import * as ui from "../odin_server/ui.js"; // @ts-ignore
import * as ws from "../odin_server/ws.js"; // @ts-ignore
import * as odinCesium from "../odin_cesium/odin_cesium.js";
const MODULE_PATH = util.asset_path(import.meta.url);
const POINT_TYPE = "testPoint";
const OASIS_SETTINGS = "oasisSettings";
const POINT_DETAILS = "pointDetails";
ws.addWsHandler(MODULE_PATH, handleWsMessages);
//--- display params we can change from config file can be extracted here as Consts
let selectedOasisPoint = "";
let oasis_data = [];
createIcon();
createSettingsWindow();
createDetailsWindow();
initPowerLineDetailsView();
odinCesium.setEntitySelectionHandler(oasisPointSelection);
odinCesium.initLayerPanel(OASIS_SETTINGS, config, toggleOasisPoints);
odinCesium.initLayerPanel(POINT_DETAILS, config, () => null);
let pointDataSource = null;
if (config.layer.show) {
    console.log("should show points load");
    initOasisPoints();
}
console.log("ui_bcit_smart initialized");
function createIcon() {
    // return ui.Icon("./asset/bcit_smart/powerline_icon.svg", (e)=> ui.toggleWindow(e,LINE_SETTINGS));
    return ui.Icon("./asset/odin_cesium/globe.svg", (e) => ui.toggleWindow(e, OASIS_SETTINGS));
}
function createSettingsWindow() {
    return ui.Window("Test Oasis Data", OASIS_SETTINGS, "./asset/bcit_smart/button_svg.svg")(ui.LayerPanel(OASIS_SETTINGS, toggleShowPoints));
}
function createDetailsWindow() {
    const testSpan = document.createElement("span");
    testSpan.id = "point-details-name";
    return ui.Window("Point Details", POINT_DETAILS, "./asset/bcit_smart/button_svg.svg")(testSpan, ui.Panel("data sets", true, "oasis-details-window")(ui.CheckBox("show lines", toggleShowPoints, "lines"), ui.List("powerlines.selectedPowerline", 3, () => console.log("When is this called?"))));
}
function toggleShowPoints(event) {
    let cb = ui.getCheckBox(event.target);
    console.log(event.target);
    if (cb) {
        toggleOasisPoints(ui.isCheckBoxSelected(cb));
    }
}
function toggleOasisPoints(showLines) {
    console.log("Toggle Test Lines" + showLines);
    if (pointDataSource === null) {
        initOasisPoints();
    }
    pointDataSource.show = showLines ?? true;
    odinCesium.requestRender();
}
function clearEntities() {
    if (pointDataSource) {
        pointDataSource.entities.removeAll();
    }
}
function oasisPointSelection() {
    let sel = odinCesium.getSelectedEntity();
    if (sel && sel._type && sel._type == POINT_TYPE) {
        console.log("Window should open");
        ;
        console.log("Selection", sel);
        ui.showWindow(POINT_DETAILS);
        ui.setWindowLocation(ui.getWindow(POINT_DETAILS), 200, 200);
        ui.setListItems("powerlines.selectedPowerline", [sel]);
        selectedOasisPoint = sel.name;
        let detailsWindow = ui.getWindow(POINT_DETAILS);
        console.log("detailsWindiw", detailsWindow);
        // let children = detailsWindow.children;
        // children[0].
        // Replace the title in the `ui_titlebar`
        const titleBar = detailsWindow.querySelector(".ui_titlebar");
        if (titleBar) {
            const titleTextNode = Array.from(titleBar.childNodes).find(node => node.nodeType === Node.TEXT_NODE);
            if (titleTextNode) {
                titleTextNode.nodeValue = selectedOasisPoint;
            }
        }
        // Replace the content in the `ui_window_content`
        const content = detailsWindow.querySelector(".ui_window_content");
        if (content) {
            // Clear the existing content
            content.innerHTML = "";
            // Create a canvas for the Chart.js chart
            const canvas = document.createElement("canvas");
            canvas.style.height = "400px"; // Set the height
            canvas.style.width = "1000px"; // Set the width
            canvas.id = "chartCanvas";
            content.appendChild(canvas);
            let timeStamps = oasis_data.map(row => row.timestamp);
            // console.log(firstDate);
            const labels = oasis_data;
            // Initialize Chart.js
            const ctx = canvas.getContext("2d");
            const myChart = new Chart(ctx, {
                type: "line", // Example: Bar chart
                data: {
                    labels: oasis_data.map(row => row.timestamp),
                    datasets: [{
                            label: "Battery Group 1.DC Power",
                            data: oasis_data.map(row => row["Battery Group 1.DC Power"]),
                            borderColor: [
                                'rgba(255, 99, 132, 1)',
                            ],
                        }]
                },
                options: {
                    responsive: true,
                    plugins: {
                        title: {
                            display: true,
                            text: "Oasis Data Line Chart"
                        },
                        zoom: {
                            pan: {
                                enabled: true,
                                mode: "x"
                            },
                            zoom: {
                                wheel: {
                                    enabled: true, // Enable zooming with mouse wheel
                                },
                                pinch: {
                                    enabled: true
                                },
                                mode: "x"
                            }
                        }
                    },
                    scales: {
                        x: {
                            type: "time", // Use time scale for X-axis
                            time: {
                                parser: "YYYY-MM-DDTHH:mm:ssZ",
                                tooltipFormat: "ll HH:mm", // Tooltip formatting
                                unit: "day" // Adjust time unit
                            },
                            title: {
                                display: true,
                                text: "Timestamp"
                            }
                        },
                        y: {
                            title: {
                                display: true,
                                text: "Power (kW)"
                            },
                            min: -5,
                            max: 5,
                        }
                    }
                }
            });
        }
    }
    console.log("selected entity: ");
    console.log(sel);
}
// function BuildOasisPointDetailsVisualization(pointName: string, window: HTMLElement<div> ) {
// }
//--- data messages
function handleWsMessages(msgType, msg) {
    console.log("GOT A WS MESSAGE ON OASIS");
    console.log(msgType);
    console.log(msg);
    switch (msgType) {
        case "oasis_data":
            handleOasisDataSet(msg);
            break;
    }
}
function handleOasisDataSet(new_oasis_data) {
    console.log("Going add the oasisData");
    // oasis_data = new_oasis_data;
    oasis_data = new_oasis_data.slice(0, Math.min(new_oasis_data.length, 365));
    // if (pointDataSource === null) {
    //     pointDataSource = new Cesium.CustomDataSource(POINT_TYPE);
    // }
    // // Think about how to handle updates as in replacing data // updating already received data etc..
    // // For now just going to replace everything on update
    // pointDataSource.entities.removeAll();
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
function initOasisPoints() {
    const point = [-122.9994, 49.2497];
    if (!pointDataSource) {
        pointDataSource = new Cesium.CustomDataSource("oasisPoints");
        odinCesium.addDataSource(pointDataSource);
    }
    const pointEntity = new Cesium.Entity({
        position: Cesium.Cartesian3.fromDegrees(point[0], point[1]),
        point: {
            pixelSize: 10,
            color: Cesium.Color.RED,
        },
        description: "Oasis EV Chargers", // Tooltip text for the point
        name: "Oasis EV Chargers", // Name of the entity
        _type: POINT_TYPE,
        label: {
            text: "Oasis Point",
            font: config.font,
            fillColor: config.outlineColor,
            showBackground: true,
            backgroundColor: config.labelBackground,
            //heightReference: Cesium.HeightReference.CLAMP_TO_GROUND,
            horizontalOrigin: Cesium.HorizontalOrigin.LEFT,
            verticalOrigin: Cesium.VerticalOrigin.TOP,
            pixelOffset: new Cesium.Cartesian2(5, 5),
            scaleByDistance: new Cesium.NearFarScalar(500.0, 1.0, // Full visibility at 100 meters
            2000.0, 0.4 // Half visibility at 1000 meters
            )
        }
    });
    const point2 = [-122.9985, 49.2493];
    const pointEntity2 = new Cesium.Entity({
        position: Cesium.Cartesian3.fromDegrees(point2[0], point2[1]),
        point: {
            pixelSize: 10,
            color: Cesium.Color.RED,
        },
        description: "Oasis Point Example", // Tooltip text for the point
        name: "Oasis Point", // Name of the entity
        _type: POINT_TYPE,
        label: {
            text: "Oasis Battery Bank",
            font: config.font,
            fillColor: config.outlineColor,
            showBackground: true,
            backgroundColor: config.labelBackground,
            //heightReference: Cesium.HeightReference.CLAMP_TO_GROUND,
            horizontalOrigin: Cesium.HorizontalOrigin.LEFT,
            verticalOrigin: Cesium.VerticalOrigin.TOP,
            pixelOffset: new Cesium.Cartesian2(5, 5),
            scaleByDistance: new Cesium.NearFarScalar(500.0, 1.0, // Full visibility at 100 meters
            2000.0, 0.4 // Half visibility at 1000 meters
            )
        }
    });
    // Add the point entity to the data source
    pointDataSource.entities.add(pointEntity);
    pointDataSource.entities.add(pointEntity2);
    // Request a render update (if necessary)
    odinCesium.requestRender();
}
