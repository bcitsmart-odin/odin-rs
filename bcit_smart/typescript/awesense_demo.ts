// These are just for VSCode's intellisense, comment them out when compiling or won't work correctly
// I could not figure out a good way to handle how the files change places when run
// @ts-ignore
// declare const util: typeof import("../../odin_server/assets/ui_util.js"); // @ts-ignore
// declare const ws: typeof import("../../odin_server/assets/ws.js"); // @ts-ignore
// declare const ui: typeof import("../../odin_server/assets/ui.js"); // @ts-ignore
// declare const odinCesium: typeof import("../../odin_cesium/assets/odin_cesium.js");

// @ts-ignore
import { config } from "./awesense_config.js"; // @ts-ignore
import * as util from "../odin_server/ui_util.js";// @ts-ignore
import * as ui from "../odin_server/ui.js"; // @ts-ignore
import * as ws from "../odin_server/ws.js"; // @ts-ignore
import * as odinCesium from "../odin_cesium/odin_cesium.js";

declare const Cesium: typeof import("cesium");

const MODULE_PATH = util.asset_path(import.meta.url);

const POINT_TYPE = "awesensePoint";
const AWESENSE_SETTINGS = "awesenseSettings";
const GRID_ELEMENT_DETAILS = "awesenseElementDetails";

const AWESENSE_DEMO_NAME = "Awesense Demo Data";

const AWESENSE_CAMERA_POSITION = Cesium.Cartesian3.fromDegrees(-122.9996, 49.2494, 610);

ws.addWsHandler( MODULE_PATH, handleWsMessages);

//--- display params we can change from config file can be extracted here as Consts

let selectedGridElement = null;

createAwesenseIcon();
createAwesenseSettingsWindow();
createAwesenseDetailsWindow();

odinCesium.setEntitySelectionHandler(awesensePointSelection);

odinCesium.initLayerPanel(AWESENSE_SETTINGS, config, toggleAwesensePoints);
odinCesium.initLayerPanel(GRID_ELEMENT_DETAILS, config, () => null);

if (config.layer.show) {
    // initOasisPoints();
}
console.log("ui_bcit_smart initialized");


function createAwesenseIcon() {
    return ui.Icon("./asset/odin_cesium/globe.svg", (e)=> ui.toggleWindow(e,AWESENSE_SETTINGS));
}

/**
 *  Window that opens when you click on an Energy Oasis Icon.
 *  Has controls for the Energy Oasis Points.
 */
function createAwesenseSettingsWindow() {
    return ui.Window("Test Oasis Data", AWESENSE_SETTINGS, "./asset/bcit_smart/button_svg.svg")(
        ui.LayerPanel(AWESENSE_SETTINGS, checkboxToggleShowPoints),
    );
}

/**
 *  Window that opens when you click on an Energy Oasis point of interest
 *  Contents of this window get changed by the chart creation
 */
function createAwesenseDetailsWindow() {
    // const testSpan = document.createElement("span");
    // testSpan.id = "point-details-name";
    return ui.Window("Point Details", GRID_ELEMENT_DETAILS, "./asset/bcit_smart/button_svg.svg")(
        // testSpan,
        ui.Panel("data sets", true, "oasis-details-window")(
            ui.CheckBox("show lines", checkboxToggleShowPoints, "lines"),
            ui.List("powerlines.selectedPowerline", 3, () => console.log("When is this called?")),
        )
    );
}

function checkboxToggleShowPoints(event) {
    const cb = ui.getCheckBox(event.target);
    if (cb) {
        toggleAwesensePoints( ui.isCheckBoxSelected(cb));
    }
}

function toggleAwesensePoints(showLines) {
    if (awesenseElementSource === null) {
        // initOasisPoints();
    }
    awesenseElementSource.show = showLines ?? true;
    odinCesium.requestRender();
}

const SUPPORTED_ELEMENT_TYPES = ["Meter", "Photovoltatic", "Fuse", "CircuitBreaker", "Transformer", "Battery", "Pole", "Switch", "EVCharger", "ACLineSegment"];

/**
 * Registered with odinCessium to be called when an entity is clicked.
 * Checks to see if it was entity it is responsible for (Awesense Grid Elements) 
 */
function awesensePointSelection() {
    const sel = odinCesium.getSelectedEntity();
    if (sel && sel._type && SUPPORTED_ELEMENT_TYPES.includes(sel._type)) {
        ui.showWindow(GRID_ELEMENT_DETAILS);
        ui.setWindowLocation(ui.getWindow(GRID_ELEMENT_DETAILS), 200, 200);

        selectedGridElement = sel.name;
        const detailsWindow : HTMLDivElement = ui.getWindow(GRID_ELEMENT_DETAILS);
        buildGridElementDetailsVisualization(selectedGridElement, detailsWindow);
    }
    console.log("selected entity:", sel);
}

/**
 * Builds the details window for the selected Awesense Grid Element
 */
function buildGridElementDetailsVisualization(pointName: string, window: HTMLDivElement ) {
    const titleBar = window.querySelector(".ui_titlebar");
    if (titleBar) {
        const titleTextNode = Array.from(titleBar.childNodes).find(node => node.nodeType === Node.TEXT_NODE);
        if (titleTextNode) {
            titleTextNode.nodeValue = pointName;
        }
    }

    // Replace the content in the `ui_window_content`
    const content = window.querySelector(".ui_window_content");
    if (content) {
        // Clear the existing content
        content.innerHTML = "";

        // List out the different properties of the selected grid element
        const gridElement = grid_element_list.find(element => element.grid_element_id === pointName);
        console.log("Selected Grid Element: ", gridElement);
        if (gridElement) {
            const propertiesHTML = Object.entries(gridElement).map(([key, value]) => {
                const displayValue = value !== null && value !== undefined ? value : "N/A";
                const propertyRow = document.createElement("div");
                propertyRow.className = "property-row";
    
                const propertyKey = document.createElement("span");
                propertyKey.className = "property-key";
                propertyKey.textContent = `${key}:`;
    
                const propertyValue = document.createElement("span");
                propertyValue.className = "property-value";
                propertyValue.textContent = displayValue;
    
                propertyRow.appendChild(propertyKey);
                propertyRow.appendChild(propertyValue);
    
                return propertyRow;
            });
    
            const gridElementProperties = 
                ui.Panel("grid-element-properties", true)(
                    ...propertiesHTML
                );
            ;
            content.append(gridElementProperties);

            // Add a panel that has toggles for showing different traces, only 1 active at a time
            // Create a parent div for the buttons
            const buttonContainer = document.createElement("div");
            buttonContainer.className = "trace-button-container";

            // Generate buttons and append them to the container
            ["Source", "Down", "Same Voltage", "Connected"].forEach(traceName => {
                const traceButton = document.createElement("button");
                traceButton.className = "trace-button";
                traceButton.textContent = traceName;

                // Add event listener to toggle traces
                traceButton.addEventListener("click", () => {
                    // Deactivate all trace buttons
                    const buttons = buttonContainer.querySelectorAll(".trace-button");
                    buttons.forEach(button => button.classList.remove("active"));

                    // Activate the clicked button
                    traceButton.classList.add("active");

                    // Logic to handle the display of traces goes here
                    console.log(`Activated ${traceName} for ${gridElement.grid_element_id}`);
                    ws.sendWsMessage(
                        "bcit_smart/awesense_demo",
                        "trace_request",
                        { trace_name: traceName, grid_element_id: gridElement.grid_element_id }
                    );
                });

                // Append the button to the container
                buttonContainer.appendChild(traceButton);
            });

            const tracePanel = ui.Panel("trace-panel", true)(
                buttonContainer
            );

            content.append(tracePanel);
            
        }

        ui.initializeWindow(window);


    }
}

function handleWsMessages(msgType, msg) {
    // console.log("ws message received by oasis_points.js, type: ", msgType);
    // console.log(msg);
    switch (msgType) {
        case "awesense_element_list": handleAwesenseElementList(msg); break;
        case "awesense_trace_response": handleAwesenseTraceResponse(msg); break;
        default: {
            console.log("Unknown message type: ", msgType);
            console.log(msg);
        };
    }
}

interface GeometryType {
    Point?: [number, number];
    LineString?: [number, number][];
}

interface GridElement {
    grid_id?: string;
    grid_element_id?: string;
    type_?: string; // Use `type_` to avoid conflict with `type` keyword
    customer_type?: string;
    phases?: string;
    is_underground?: boolean;
    is_producer?: boolean;
    is_consumer?: boolean;
    is_switchable?: boolean;
    switch_is_open?: boolean;
    terminal1_cn?: string;
    terminal2_cn?: string;
    power_flow_direction?: string;
    upstream_grid_element_id?: string;
    geometry?: GeometryType; // Custom type for handling geometry
    meta?: Record<string, any>; // JSONB field is represented as an object
}

let grid_element_list = [] as GridElement[];

/**
 * Parses the Oasis Data in a way convenient to use in line chart
 */
function handleAwesenseElementList(new_awesense_data: GridElement[]) {
    grid_element_list = new_awesense_data;
    buildAwesenseElementsDataSource();
}

let traceDataSource = null;

function handleAwesenseTraceResponse(response: GridElement[]) {
    // console.log("Received trace response: ", response);
    if (!traceDataSource) {
        traceDataSource = new Cesium.CustomDataSource("traceData");
        odinCesium.addDataSource(traceDataSource);
    } else {
        traceDataSource.entities.removeAll();
    }

    // Create highlighted lines for the ACLineSegments and total up all the other element types
    let elementCounts = {};
    response.forEach((element) => {
        if (element.type_ === "ACLineSegment") {
            const lineEntity = new Cesium.Entity({
                polyline: {
                    positions: Cesium.Cartesian3.fromDegreesArray(element.geometry.LineString.flat()),
                    width: 8,
                    material: Cesium.Color.BLUE,
                },
                description: element.grid_element_id, // Tooltip text for the point
                name: element.grid_element_id, // Name of the entity
                _type: element.type_, // Custom property to identify the entity type
            } as any);

            traceDataSource.entities.add(lineEntity);
        } else {
            elementCounts[element.type_] = (elementCounts[element.type_] || 0) + 1;
        }
    });

    console.log("Element Counts: ", elementCounts);

    // Create a summary of the trace response
    // TODO

    odinCesium.requestRender();
}

let awesenseElementSource;

const iconMap = {
    Meter: "./asset/bcit_smart/icons/meter.svg",
    Photovoltatic: "./asset/bcit_smart/icons/solar-panel.svg",
    Fuse: "./asset/bcit_smart/icons/fuse.svg",
    CircuitBreaker: "./asset/bcit_smart/icons/switch.svg",
    Transformer: "./asset/bcit_smart/icons/transformer.svg",
    Battery: "./asset/bcit_smart/icons/battery.svg",
    Pole: "./asset/bcit_smart/icons/pole.svg",
    Switch: "./asset/bcit_smart/icons/switch.svg",
    EVCharger: "./asset/bcit_smart/icons/electric-station.svg",
}

const phaseToColor = {
    A: '#FF0000', // Red
    B: '#00FF00', // Green
    C: '#0000FF', // Blue
    ABC: '#FFFF00', // Yellow
};

const svgCache = {}; // Cache for modified SVGs

async function getColoredSvg(type_, fillColor) {
    // Initialize cache for the type if it doesn't exist
    if (!svgCache[type_]) { svgCache[type_] = {}; }

    // Check if the modified SVG is already cached
    if (svgCache[type_][fillColor]) {
        return svgCache[type_][fillColor];
    }

    const icon = iconMap[type_];
    if (!icon) { return null; }

    const response = await fetch(icon);
    if (!response.ok) {
        throw new Error(`Failed to fetch SVG for type "${type_}"`);
    }

    let svgContent = await response.text();
    svgContent = svgContent
                    .replace(/fill="#000000"/g, `fill="${fillColor}"`)
                    .replace(/stroke="#000000"/g, `stroke="${fillColor}"`);

    // Encode the modified SVG as a data URI
    const svgUri = `data:image/svg+xml;base64,${btoa(svgContent)}`;

    svgCache[type_][fillColor] = svgUri;

    return svgUri;
}

async function buildAwesenseElementsDataSource() {
    if (!awesenseElementSource) {
        awesenseElementSource = new Cesium.CustomDataSource("awesensePoints");

        // No clustering for now
        awesenseElementSource.clustering.enabled = false;
        awesenseElementSource.clustering.pixelRange = 11; 
        awesenseElementSource.clustering.minimumClusterSize = 2;
        
        awesenseElementSource.clustering.clusterEvent.addEventListener((clusteredEntities, cluster) => {
            cluster.label.show = true;
            cluster.label.text = clusteredEntities.length.toLocaleString();
            cluster.label.pixelOffset = new Cesium.Cartesian2(8, 5);
            cluster.label.scaleByDistance = new Cesium.NearFarScalar(
                500.0, 1.0,
                20000.0, 0.4
            );
            cluster.billboard.show = false;
            cluster.point.show = true;
            cluster.point.pixelSize = 15;
            cluster.point.color = Cesium.Color.YELLOW;
            cluster.point.scaleByDistance = new Cesium.NearFarScalar(
                500.0, 1.0,
                20000.0, 0.4
            );

            const clusterId = {
                isCluster: true,
                clusteredEntities: clusteredEntities,
                label: cluster.label.text,
            }

            cluster.point.id = clusterId;
            cluster.label.id = clusterId;
        });

        // Normal Entity selection not working for clusters adding a custom select handler it check if this cluster was clicked
        odinCesium.setEntitySelectionHandler((click) => {
            const pickedObject = odinCesium.viewer.scene.pick(click.position);
            if (Cesium.defined(pickedObject) && pickedObject.id && pickedObject.id.isCluster) {
                // odinCesium.zoomTo(AWESENSE_CAMERA_POSITION);
                console.log("Awesense Cluster Clicked", pickedObject.id);
                pickedObject.id.clusteredEntities.forEach((entity) => {
                    console.log(entity.name);
                });
            }
        });

        odinCesium.addDataSource(awesenseElementSource);
    }

    // Clear out any existing entities
    awesenseElementSource.entities.removeAll();

    grid_element_list.forEach(async (element, index) => {
        if (element.geometry && element.geometry.Point) {
            const icon = iconMap[element.type_] || null;

            // Going to skip the point if it doesn't have a valid icon for now
            if (!icon) { return; }

            const coloredSvg = await getColoredSvg(element.type_, element.phases ? phaseToColor[element.phases] : "#000000");

            const pointEntity = new Cesium.Entity({
                position: Cesium.Cartesian3.fromDegrees(element.geometry.Point[0], element.geometry.Point[1]),
                point: icon ? undefined : {
                    pixelSize: 10,
                    color: Cesium.Color.RED,
                },
                billboard: icon ? {
                    // image: iconMap[element.type_],
                    image: coloredSvg,
                    width: 32,
                    height: 32,
                    scaleByDistance: new Cesium.NearFarScalar(
                        500.0, 1.0,
                        2000.0, 0.4
                    )
                } : undefined,

                description: element.grid_element_id, // Tooltip text for the point
                name: element.grid_element_id, // Name of the entity
                _type: element.type_, // Custom property to identify the entity type
                label: {
                    text: element.type_ === "Meter" ? element.grid_element_id : undefined,
                    font: config.font,
                    fillColor: config.outlineColor,
                    showBackground: true,
                    backgroundColor: config.labelBackground,
                    horizontalOrigin: Cesium.HorizontalOrigin.LEFT,
                    verticalOrigin:  Cesium.VerticalOrigin.TOP,
                    pixelOffset: new Cesium.Cartesian2(-5, 5),
                    scaleByDistance: new Cesium.NearFarScalar(
                        500.0, 1.0, // Full visibility
                        2000.0, 0.4 // Half visibility
                    )
                }
            } as any);

            awesenseElementSource.entities.add(pointEntity);
        } else if (element.geometry && element.geometry.LineString) {
            const lineEntity = new Cesium.Entity({
                polyline: {
                    positions: Cesium.Cartesian3.fromDegreesArray(element.geometry.LineString.flat()),
                    width: 2,
                    material: Cesium.Color.BLACK,
                },
                description: element.grid_element_id, // Tooltip text for the point
                name: element.grid_element_id, // Name of the entity
                _type: element.type_, // Custom property to identify the entity type
            } as any);

            awesenseElementSource.entities.add(lineEntity);
        }
    });

    odinCesium.requestRender();
}