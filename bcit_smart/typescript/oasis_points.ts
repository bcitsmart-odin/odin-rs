// These are just for VSCode's intellisense, comment them out when compiling or won't work correctly
// I could not figure out a good way to handle how the files change places when run
// @ts-ignore
// declare const util: typeof import("../../odin_server/assets/ui_util.js"); // @ts-ignore
// declare const ws: typeof import("../../odin_server/assets/ws.js"); // @ts-ignore
// declare const ui: typeof import("../../odin_server/assets/ui.js"); // @ts-ignore
// declare const odinCesium: typeof import("../../odin_cesium/assets/odin_cesium.js");

// @ts-ignore
import { config } from "./oasis_config.js"; // @ts-ignore
import * as util from "../odin_server/ui_util.js";// @ts-ignore
import * as ui from "../odin_server/ui.js"; // @ts-ignore
import * as ws from "../odin_server/ws.js"; // @ts-ignore
import * as odinCesium from "../odin_cesium/odin_cesium.js";

declare const Cesium: typeof import("cesium");
declare const Chart: typeof import("chart.js");
import type { ChartOptions, ChartScales, LinearScale, TimeScale } from "chart.js";

type OasisDataTypeFromServer = {
    "BESS.DC Power": number | null, 
    "Battery Group 1.DC Power": number | null,
    "Battery Group 2.DC Power": number | null,
    "Battery Group 3.DC Power": number | null,
    "Battery Group 4.DC Power": number | null,
    "Inverter.Active Power": number | null,
    "L2 7650 Meter.Active Power": number | null,
    "OASIS POI.Active Power": number | null,
    "PV.DC Power": number | null,
    "rowNum": number,
    "timestamp": string,
}

type OasisDataType = {
    [K in (typeof OASIS_DATA_TITLES)[number]]: (number | null)[];
};

type OasisLineChartDataType = {
    timestamp: string[]
    data: OasisDataType
}

const MODULE_PATH = util.asset_path(import.meta.url);

const POINT_TYPE = "oasisPoint";
const OASIS_SETTINGS = "oasisSettings";
const POINT_DETAILS = "oasisPointDetails";

const ENERGY_OASIS_NAME = "BCIT's Energy Oasis";

const BCIT_OASIS_CAMERA_POSITION = Cesium.Cartesian3.fromDegrees(-122.9996, 49.2494, 610);

const OASIS_DATA_TITLES = [
    "BESS.DC Power", 
    "Battery Group 1.DC Power",
    "Battery Group 2.DC Power",
    "Battery Group 3.DC Power",
    "Battery Group 4.DC Power",
    "Inverter.Active Power",
    // "L2 7650 Meter.Active Power", // No data so skipping this one
    "OASIS POI.Active Power",
    "PV.DC Power",
] as const;

// I should rethink this charts field so it is not so fragily tied to the OASIS_DATA_TITLES order/length
const OASIS_POINTS_CONFIG = [
    { name: "Energy Oasis EV Chargers", lonlat: [-122.999400, 49.249700], charts: [7] },
    { name: "Energy Oasis BESS", lonlat: [-122.998640, 49.249430], charts: [0,1,2,3,4] },
    { name: "Energy Oasis Inverter", lonlat: [-122.998470, 49.249430], charts: [5] },
    { name: "Energy Oasis Substation K", lonlat: [-123.000600, 49.248900], charts: [6] },
]

const CHART_COLORS = [
    { label: "Red", rgba: "rgba(255, 99, 132, 1)" },
    { label: "Blue", rgba: "rgba(54, 162, 235, 1)" },
    { label: "Green", rgba: "rgba(34, 139, 34, 1)" },
    { label: "Yellow", rgba: "rgba(255, 255, 102, 1)" },
    { label: "Purple", rgba: "rgba(153, 102, 255, 1)" },
    { label: "Orange", rgba: "rgba(255, 159, 64, 1)" },
    { label: "Teal", rgba: "rgba(64, 224, 208, 1)" },
    { label: "Pink", rgba: "rgba(255, 182, 193, 1)" },
    { label: "Gray", rgba: "rgba(128, 128, 128, 1)" },
    { label: "Cyan", rgba: "rgba(0, 255, 255, 1)" }
];

ws.addWsHandler( MODULE_PATH, handleWsMessages);

//--- display params we can change from config file can be extracted here as Consts

let oasisPointsDataSource = null;
let oasis_data: OasisLineChartDataType;
let selectedOasisPoint = "";

createOasisIcon();
createOasisSettingsWindow();
createOasisDetailsWindow();

odinCesium.setEntitySelectionHandler(oasisPointSelection);

odinCesium.initLayerPanel(OASIS_SETTINGS, config, toggleOasisPoints);
odinCesium.initLayerPanel(POINT_DETAILS, config, () => null);

if (config.layer.show) {
    initOasisPoints();
}
console.log("ui_bcit_smart initialized");


function createOasisIcon() {
    return ui.Icon("./asset/odin_cesium/globe.svg", (e)=> ui.toggleWindow(e,OASIS_SETTINGS));
}

/**
 *  Window that opens when you click on an Energy Oasis Icon.
 *  Has controls for the Energy Oasis Points.
 */
function createOasisSettingsWindow() {
    return ui.Window("Test Oasis Data", OASIS_SETTINGS, "./asset/bcit_smart/button_svg.svg")(
        ui.LayerPanel(OASIS_SETTINGS, checkboxToggleShowPoints),
    );
}

/**
 *  Window that opens when you click on an Energy Oasis point of interest
 *  Contents of this window get changed by the chart creation
 */
function createOasisDetailsWindow() {
    const testSpan = document.createElement("span");
    testSpan.id = "point-details-name";
    return ui.Window("Point Details", POINT_DETAILS, "./asset/bcit_smart/button_svg.svg")(
        testSpan,
        ui.Panel("data sets", true, "oasis-details-window")(
            ui.CheckBox("show lines", checkboxToggleShowPoints, "lines"),
            ui.List("powerlines.selectedPowerline", 3, () => console.log("When is this called?")),
        )
    );
}

function checkboxToggleShowPoints(event) {
    const cb = ui.getCheckBox(event.target);
    if (cb) {
        toggleOasisPoints( ui.isCheckBoxSelected(cb));
    }
}

function toggleOasisPoints(showLines) {
    if (oasisPointsDataSource === null) {
        initOasisPoints();
    }
    oasisPointsDataSource.show = showLines ?? true;
    odinCesium.requestRender();
}

/**
 * Registered with odinCessium to be called when an entity is clicked.
 * Checks to see if it was entity it is responsible for (Energy Oasis Points) 
 */
function oasisPointSelection() {
    const sel = odinCesium.getSelectedEntity();
    if (sel && sel._type && sel._type == POINT_TYPE) {
        ui.showWindow(POINT_DETAILS);
        ui.setWindowLocation(ui.getWindow(POINT_DETAILS), 200, 200);

        selectedOasisPoint = sel.name;
        const detailsWindow : HTMLDivElement = ui.getWindow(POINT_DETAILS);
        buildOasisPointDetailsVisualization(selectedOasisPoint, detailsWindow);
    }
    // console.log("selected entity:", sel);
}

/**
 * Creates the Charts showing the historical Energy Oasis Data.
 * Currently all points show the same data.
 */
function buildOasisPointDetailsVisualization(pointName: string, window: HTMLDivElement ) {
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

        // Create a canvas for the Chart.js chart
        const canvas = document.createElement("canvas");
        canvas.style.height = "400px";
        canvas.style.width = "1000px";
        canvas.id = "chartCanvas";
        content.appendChild(canvas);

        // Initialize Chart.js
        const ctx = canvas.getContext("2d");

        const chartOptions: ChartOptions = {
            responsive: true,
            hover: { mode: "nearest" },
            plugins: {
                title: { display: true, text: "Energy Oasis Data Line Chart" },
                zoom: {
                    pan: { enabled: true, mode: "x" },
                    zoom: {
                        wheel: { enabled: true },
                        pinch: { enabled: true },
                        mode: "x",
                    },
                },
            },
            scales: {
                x: {
                    type: "time",
                    time: {
                        parser: "YYYY-MM-DDTHH:mm:ssZ",
                        tooltipFormat: "ll HH:mm",
                        unit: "day",
                        displayFormats: { day: "MMM D, YYYY" }, // label format
                    },
                    title: { display: true, text: "Timestamp" },
                } as TimeScale,
                y: {
                    beginAtZero: false, // Allow Y-axis to start dynamically
                    suggestedMin: 0, // So that graph starting point is never greater than 0
                    title: { display: true, text: "Power (kW)" },
                    ticks: {
                        callback: (value) => `${value} kW`, // Format Y-axis labels
                    },
                    afterDataLimits: (axis) => { // This centers the graph at 0
                        const maxAbsValue = Math.max(Math.abs(axis.max || 0), Math.abs(axis.min || 0));
                        axis.min = -maxAbsValue;
                        axis.max = maxAbsValue;
                    },
                } as LinearScale,
            } as ChartScales,
        };

        const visibleCharts = OASIS_POINTS_CONFIG.find((point) => point.name === pointName)?.charts ?? [];

        const myChart = new Chart(ctx, {
            type: "line",
            data: {
                labels: oasis_data.timestamp,
                datasets: OASIS_DATA_TITLES.filter((title, index) => visibleCharts.includes(index)).map((title, index) => ({
                    label: title,
                    data: oasis_data.data[title] ?? [],
                    borderColor: CHART_COLORS[index].rgba,
                    borderWidth: 1.5,
                    pointRadius: 0,
                    pointHoverRadius: 5,
                    pointHitRadius: 8,
                }))
            },
            options: chartOptions
        });
    }
}

function handleWsMessages(msgType, msg) {
    // console.log("ws message received by oasis_points.js, type: ", msgType);
    // console.log(msg);
    switch (msgType) {
        case "oasis_data": handleOasisDataSet(msg); break;
    }
}

/**
 * Parses the Oasis Data in a way convenient to use in line chart
 */
function handleOasisDataSet(new_oasis_data: OasisDataTypeFromServer[]) {
    const parsedChartData: OasisLineChartDataType = {
        timestamp: [],
        data: Object.fromEntries(OASIS_DATA_TITLES.map((title) => [title, []])) as OasisDataType,
    };

    new_oasis_data.forEach((row) => {
        parsedChartData.timestamp.push(row.timestamp);
        OASIS_DATA_TITLES.forEach((title) => {
            parsedChartData.data[title].push(row[title]);
        });
    });

    oasis_data = parsedChartData;
}

function initOasisPoints() {
    buildOasisPointsDataSource();

    OASIS_POINTS_CONFIG.forEach((point, index) => {
        const pointEntity = new Cesium.Entity({
            position: Cesium.Cartesian3.fromDegrees(point.lonlat[0], point.lonlat[1]),
            point: {
                pixelSize: 10,
                color: Cesium.Color.RED,
            },
            description: point.name, // Tooltip text for the point
            name: point.name, // Name of the entity
            _type: POINT_TYPE,
            label: {
                text: point.name,
                font: config.font,
                fillColor: config.outlineColor,
                showBackground: true,
                backgroundColor: config.labelBackground,
                //heightReference: Cesium.HeightReference.CLAMP_TO_GROUND,
                horizontalOrigin: index != 1 ? Cesium.HorizontalOrigin.LEFT : Cesium.HorizontalOrigin.RIGHT,
                verticalOrigin:  Cesium.VerticalOrigin.TOP,
                pixelOffset: new Cesium.Cartesian2(index != 1 ? 5 : -5, 5),
                scaleByDistance: new Cesium.NearFarScalar(
                    500.0, 1.0, // Full visibility
                    2000.0, 0.4 // Half visibility
                )
            }
        } as any);

        oasisPointsDataSource.entities.add(pointEntity);
    });

    odinCesium.requestRender();
}

function buildOasisPointsDataSource() {
    if (!oasisPointsDataSource) {
        oasisPointsDataSource = new Cesium.CustomDataSource("oasisPoints");
        
        // Adjust the clustering if more points are added we want this to be all or nothing
        oasisPointsDataSource.clustering.enabled = true;
        oasisPointsDataSource.clustering.pixelRange = 8; 
        oasisPointsDataSource.clustering.minimumClusterSize = 4;
        
        oasisPointsDataSource.clustering.clusterEvent.addEventListener((clusteredEntities, cluster) => {
            cluster.label.show = true;
            cluster.label.text = ENERGY_OASIS_NAME;
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
            if (Cesium.defined(pickedObject) && pickedObject.id && pickedObject.id.label == ENERGY_OASIS_NAME) {
                odinCesium.zoomTo(BCIT_OASIS_CAMERA_POSITION);
            }
        });

        odinCesium.addDataSource(oasisPointsDataSource);
    }
}