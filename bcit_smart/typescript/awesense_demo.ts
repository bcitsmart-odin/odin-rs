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

const AWESENSE_GRID_ELEMENT = "awesenseGridElement";
const AWESENSE_SETTINGS = "awesenseSettings";
const GRID_ELEMENT_DETAILS = "awesenseElementDetails";

const AWESENSE_DEMO_NAME = "Awesense Demo Data";

ws.addWsHandler( MODULE_PATH, handleWsMessages);

//--- display params we can change from config file can be extracted here as Consts


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
let traceDataSource = null;

let selectedGridElement = null;
let awesenseElementSource;

const iconMap = {
    Meter: "./asset/bcit_smart/icons/meter.svg",
    Photovoltaic: "./asset/bcit_smart/icons/solar-panel.svg",
    Fuse: "./asset/bcit_smart/icons/fuse.svg",
    CircuitBreaker: "./asset/bcit_smart/icons/switch.svg",
    Transformer: "./asset/bcit_smart/icons/transformer.svg",
    Battery: "./asset/bcit_smart/icons/battery.svg",
    Pole: "./asset/bcit_smart/icons/pole.svg",
    Switch: "./asset/bcit_smart/icons/switch.svg",
    EVCharger: "./asset/bcit_smart/icons/electric-station.svg",
    ACLineSegment: "./asset/bcit_smart/icons/line.svg",
}

const phaseToColor = {
    A: '#FF0000', // Red
    B: '#00FF00', // Green
    C: '#0000FF', // Blue
    ABC: '#FFFF00', // Yellow
};

const SUPPORTED_ELEMENT_TYPES = ["Meter", "Photovoltaic", "Fuse", "CircuitBreaker", "Transformer", "Battery", "Pole", "Switch", "EVCharger", "ACLineSegment"];

const svgCache = {}; // Cache for modified SVGs
const elementVisibility = {}

buildAwesenseElementsDataSource();
createAwesenseIcon();
createAwesenseSettingsWindow();
createAwesenseDetailsWindow();

odinCesium.setEntitySelectionHandler(awesensePointSelection);

odinCesium.initLayerPanel(AWESENSE_SETTINGS, config, toggleAwesensePoints);
odinCesium.initLayerPanel(GRID_ELEMENT_DETAILS, config, () => null);

console.log("ui_awesense_demo initialized");


function createAwesenseIcon() {
    return ui.Icon("./asset/odin_cesium/globe.svg", (e)=> ui.toggleWindow(e,AWESENSE_SETTINGS));
}

/**
 *  Window that opens when you click on the Awesense Icon.
 *  Has controls for the display of awesence grid elements.
 */
function createAwesenseSettingsWindow() {
    SUPPORTED_ELEMENT_TYPES.forEach(type => {
        elementVisibility[type] = true; // Default to visible
    });

    // Callback to handle checkbox toggle
    function checkboxToggleShowPoints(type) {
        return function (event) {
            const isChecked = event.target.checked;
            elementVisibility[type] = isChecked;

            // Toggle visibility of entities of this type
            awesenseElementSource.entities.values.forEach(entity => {
                if (entity._type === type) {
                    entity.show = isChecked;
                }
            });

            odinCesium.requestRender();

            console.log(`Toggled ${type}: ${isChecked}`);
        };
    }

    // Create checkboxes for each element type
    const checkboxes = SUPPORTED_ELEMENT_TYPES.map(type => {
        const checkbox = document.createElement("input");
        checkbox.type = "checkbox";
        checkbox.checked = true; // Default to checked (visible)
        checkbox.className = "type-checkbox";
        checkbox.addEventListener("change", checkboxToggleShowPoints(type));

        const label = document.createElement("label");
        label.className = "checkbox-label";
        label.textContent = type;

        // Add an icon for each type
        const icon = document.createElement("img");
        icon.src = iconMap[type]; // Adjust path and file names
        icon.alt = `${type} icon`;
        icon.className = "type-icon";

        // Combine icon, checkbox, and label
        const container = document.createElement("div");
        container.className = "checkbox-container";
        container.appendChild(icon);
        container.appendChild(checkbox);
        container.appendChild(label);

        return container;
    });

     // Create legend container
    const legend = document.createElement("div");
    legend.className = "legend-container";

    const legendTitle = document.createElement("h3");
    legendTitle.textContent = "Phase Legend";
    legend.appendChild(legendTitle);

    Object.entries(phaseToColor).forEach(([phase, color]) => {
        const legendItem = document.createElement("div");
        legendItem.className = "legend-item";

        // Color box
        const colorBox = document.createElement("span");
        colorBox.className = "legend-color-box";
        colorBox.style.backgroundColor = color;

        // Phase label
        const phaseLabel = document.createElement("span");
        phaseLabel.className = "legend-phase-label";
        phaseLabel.textContent = phase;

        // Append to legend item
        legendItem.appendChild(colorBox);
        legendItem.appendChild(phaseLabel);

        legend.appendChild(legendItem);
    });

    // console.log("checkboxes", checkboxes);

    return ui.Window("Awesense Data Settings", AWESENSE_SETTINGS, "./asset/bcit_smart/button_svg.svg")(
        ui.LayerPanel(AWESENSE_SETTINGS, checkboxToggleShowPoints),
        ui.Panel("data sets", true, "awesense-settings-window")(
            ...checkboxes,
        ),
        ui.Panel("legend", true, "awesense-legend-window")(
            legend
        )
    );
}

/**
 *  Window that opens when you click on an Awesense Grid Element
 *  Contents of this window get changed by clicking on a different element
 */
function createAwesenseDetailsWindow() {
    return ui.Window("Point Details", GRID_ELEMENT_DETAILS, "./asset/bcit_smart/button_svg.svg")(
        ui.Panel("data sets", true, "awesense-details-window")(
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
    awesenseElementSource.show = showLines ?? true;
    odinCesium.requestRender();
}

/**
 * Registered with odinCessium to be called when an entity is clicked.
 * Checks to see if it was entity it is responsible for (Awesense Grid Elements) 
 */
function awesensePointSelection() {
    const sel = odinCesium.getSelectedEntity();
    if (sel && sel._type && SUPPORTED_ELEMENT_TYPES.includes(sel._type)) {
        ui.showWindow(GRID_ELEMENT_DETAILS);
        ui.setWindowLocation(ui.getWindow(GRID_ELEMENT_DETAILS), 100, 100);

        selectedGridElement = sel.name;
        const detailsWindow : HTMLDivElement = ui.getWindow(GRID_ELEMENT_DETAILS);
        buildGridElementDetailsVisualization(selectedGridElement, detailsWindow);
    }
    console.log("selected entity:", sel);
}

function formatGeometry(geometry: GeometryType): string {
    if ("Point" in geometry && Array.isArray(geometry.Point)) {
        const [x, y] = geometry.Point;
        return `${x.toFixed(4)}, ${y.toFixed(4)}`;
    } else if ("LineString" in geometry && Array.isArray(geometry.LineString)) {
        const formattedCoords = geometry.LineString.map(([x, y]) =>
            `[${x.toFixed(4)}, ${y.toFixed(4)}]`
        );
        return formattedCoords.join(", ");
    } else {
        return "Unsupported Geometry";
    }
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
                let displayValue = value !== null && value !== undefined ? value : "N/A";
                if (key === "geometry" && value !== null && value !== undefined) {
                    displayValue = formatGeometry(value);
                }
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

/**
 * Parses the Awesense Data in a way convenient to use in line chart
 */
function handleAwesenseElementList(new_awesense_data: GridElement[]) {
    grid_element_list = new_awesense_data;
    buildAwesenseElementsDataSource();
}


function handleAwesenseTraceResponse(response: GridElement[]) {
    // console.log("Received trace response: ", response);
    if (!traceDataSource) {
        traceDataSource = new Cesium.CustomDataSource("traceData");
        odinCesium.addDataSource(traceDataSource);
    } else {
        traceDataSource.entities.removeAll();
    }

    // Create highlighted lines for the ACLineSegments and total up all the other element types
    const elementCounts = {};
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
        }
        elementCounts[element.type_] = (elementCounts[element.type_] || 0) + 1;
    });

    console.log("Element Counts: ", elementCounts);

    // Create a summary of the trace response
    // TODO

    odinCesium.requestRender();
}

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