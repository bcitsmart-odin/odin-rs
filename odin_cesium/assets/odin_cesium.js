/**
 * Copyright © 2024, United States Government, as represented by the Administrator of 
 * the National Aeronautics and Space Administration. All rights reserved.
 *
 * The “ODIN” software is licensed under the Apache License, Version 2.0 (the "License"); 
 * you may not use this file except in compliance with the License. You may obtain a copy 
 * of the License at http://www.apache.org/licenses/LICENSE-2.0.
 *
 * Unless required by applicable law or agreed to in writing, software distributed under
 * the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND,
 * either express or implied. See the License for the specific language governing permissions
 * and limitations under the License.
 */

// the ecmascript module that is our CesiumJS interface. Not this is an async module


import { config } from "./odin_cesium_config.js";
import * as util from "../odin_server/ui_util.js";
import * as ui from "../odin_server/ui.js";
import * as ws from "../odin_server/ws.js";

const MOD_PATH = "odin_cesium::CesiumService";

ws.addWsHandler( MOD_PATH, handleWsMessages);
setCesiumContainerVisibility(false); // don't render before everybody is initialized


const UI_POSITIONS = "race-ui-positions";
const LOCAL = "local-";  // prefix for local position set names

class LayerEntry {
    constructor (wid,layerConfig,showAction) {
        this.id = layerConfig.name;    // (unique) full path: /cat/.../name

        let p = util.matchPath(this.id);
        this.name = p[2];
        this.category = p[1];

        this.config = layerConfig;     // at minimum {name,description,show}
        this.show = layerConfig.show;  // the configured initial state
        this.showAction = showAction;   // module provided function to toggle visibility of assets

        this.modulePanelCb = undefined;
        this.layerOrderCb = undefined;
    }

    setVisible(showIt) {
        this.show = showIt;
        this.showAction(showIt);
        ui.setCheckBox(this.modulePanelCb,showIt); // in the module window
        ui.setCheckBox(this.layerOrderCb, showIt);
    }
}

// don't depend on member functions as we serialize/deserialize these

class PositionSet {
    constructor (name, positions) {
        this.name = name;
        this.positions = positions;
    }
}

class Position {
    constructor (name, latDeg, lonDeg, altM) {
        this.name = name;
        this.lat = latDeg;
        this.lon = lonDeg;
        this.alt = altM;

        this.asset = undefined; // on-demand point entity
    }
}

//export var viewer = undefined;

var cameraSpec = undefined;
var lastCamera = undefined; // saved last position & orientation

var requestRenderMode = config.requestRenderMode;
var pendingRenderRequest = false;
var targetFrameRate = -1;

var layerOrder = []; // populated by initLayerPanel calls from modules
var layerOrderView = undefined; // showing the registered module layers
var layerHierarchy = [];
var layerHierarchyView = undefined;

var mouseMoveHandlers = [];
var mouseClickHandlers = [];
var mouseDblClickHandlers = [];
var terrainChangeHandlers = [];

var homePosition = undefined;
var initPosition = undefined;
var selectedPositionSet = undefined;
var positions = undefined;
var positionsView = undefined;

var isSelectedView = false;

const centerOrientation = {
    heading: Cesium.Math.toRadians(0.0),
    pitch: Cesium.Math.toRadians(-90.0),
    roll: Cesium.Math.toRadians(0.0)
};

export const ellipsoidTerrainProvider = new Cesium.EllipsoidTerrainProvider();
var terrainProvider = ellipsoidTerrainProvider; // switched on demand

if (Cesium.Ion.defaultAccessToken) {
    console.log("using configured Ion access token");
}

export const viewer = new Cesium.Viewer('cesiumContainer', {
    terrainProvider: terrainProvider,
    skyBox: false,
    infoBox: false,
    baseLayerPicker: false,  // if true primitives don't work anymore ?? 
    baseLayer: false,        // set during imageryService init
    sceneModePicker: true,
    navigationHelpButton: false,
    homeButton: false,
    timeline: false,
    animation: false,
    requestRenderMode: requestRenderMode,
});

checkImagery();

let positionSets = getPositionSets();

let dataSource = new Cesium.CustomDataSource("positions");
addDataSource(dataSource);

initTimeWindow();
initViewWindow();
initLayerWindow();

// position fields
let cameraLat = ui.getField("view.camera.latitude");
let cameraLon = ui.getField("view.camera.longitude");
let cameraAlt = ui.getField("view.camera.altitude");
let pointerLat = ui.getField("view.pointer.latitude");
let pointerLon = ui.getField("view.pointer.longitude");
let pointerElev = ui.getField("view.pointer.elevation");
let pointerUtmN = ui.getField("view.pointer.utmN");
let pointerUtmE = ui.getField("view.pointer.utmE");
let pointerUtmZ = ui.getField("view.pointer.utmZ");

setTargetFrameRate(config.targetFrameRate);
initFrameRateSlider();

if (requestRenderMode) ui.setCheckBox("view.rm", true);

setCanvasSize();
window.addEventListener('resize', setCanvasSize);

viewer.resolutionScale = window.devicePixelRatio; // 2.0
viewer.scene.fxaa = true;
//viewer.scene.globe.depthTestAgainstTerrain=true;

//showContext(); // for debugging purposes

Cesium.GeoJsonDataSource.clampToGround = true; // should this be configured?

// event listeners
viewer.camera.moveEnd.addEventListener(updateCamera);

registerMouseMoveHandler(updateMouseLocation);
viewer.scene.canvas.addEventListener('mousemove', handleMouseMove);
viewer.scene.canvas.addEventListener('click', handleMouseClick);
viewer.scene.canvas.addEventListener('dblclick', handleMouseDblClick);


// FIXME - this seems to be broken as of Cesium 105.1
viewer.scene.postRender.addEventListener(function() {
    pendingRenderRequest = false;
});

setInitialView();

var terrainProviderPromise = undefined; // set in postInitialize
var topoTerrainProvider = undefined;

console.log("ui_cesium initialized");

//--- end initialization

function showContext() {
    let canvas = viewer.canvas;
    let gl = canvas.getContext("webgl2");
    let scene = viewer.scene;
    console.log("webGL extensions: ", gl.getSupportedExtensions());
    console.log("clamp-to-height supportet:", scene.clampToHeightSupported);
    console.log("logarithmic depth buffer:", scene.logarithmicDepthBuffer, ", far/near ratio:", scene.logarithmicDepthFarToNearRatio);
}

//--- terrain handling

function getTerrainProviderPromise() {
    if (config.terrainProvider) {
        return config.terrainProvider;
    } else {
        return Cesium.createWorldTerrainAsync();
    } 
}

const ORTHO_PITCH = -Math.PI/2;
const TERRAIN_HEIGHT = 100000; // in meters

export function isOrthoView () {
    let pitch = viewer.camera.pitch;
    return Math.abs(ORTHO_PITCH - pitch) < 0.0005;
}

function useEllipsoidTerrain() {
    if (!isOrthoView()) {
        let height = viewer.camera.positionCartographic.height;
        return height > TERRAIN_HEIGHT;
    }
    return true;
}

export async function getTopoTerrainProvider() {
    return await terrainProviderPromise;
}

function toggleTerrain(event) {
    let cb = ui.getCheckBox(event.target);
    if (cb) {
        if (ui.isCheckBoxSelected(cb)) {
            switchToTopoTerrain();
        } else {
            switchToEllipsoidTerrain();
        }
    }
}

function switchToEllipsoidTerrain() {
    if (!(terrainProvider === ellipsoidTerrainProvider)) {
        terrainProvider = ellipsoidTerrainProvider;
        console.log("switching to ellipsoid terrain");
        viewer.scene.terrainProvider = terrainProvider;
        handleTerrainChange();
        //requestRender();
    }
}

async function switchToTopoTerrain() {
    if (terrainProvider === ellipsoidTerrainProvider) {
        terrainProvider = await terrainProviderPromise;
        console.log("switching to topographic terrain");
        viewer.scene.terrainProvider = terrainProvider;
        handleTerrainChange();
        //requestRender();
    }
} 

export function isUsingTopoTerrain() {
    return !(terrainProvider === ellipsoidTerrainProvider);
}

export function registerTerrainChangeHandler (handler) {
    terrainChangeHandlers.push(handler);
}

export function releaseTerrainChangeHandler (handler) {
    let idx = terrainChangeHandlers.findIndex(h => h === handler);
    if (idx >= 0) terrainChangeHandlers.splice(idx,1);
}

function handleTerrainChange() {
    let e = viewer.scene.terrainProviderChanged;
    terrainChangeHandlers.forEach( h=> h(e));
}

//--- imagery

function checkImagery() {
    // TODO - check if this works since it is recursive
    import("./imglayer.js").catch((err) => {
        console.log("no imglayer configured, using default imagery");
        const imageryProvider = Cesium.ImageryLayer.fromWorldImagery({
            style: Cesium.IonWorldImageryStyle.AERIAL_WITH_LABELS
        });
        viewer.imageryLayers.add(imageryProvider);
    });
}

function initViewWindow() {
    createViewIcon();
    createViewWindow();
    positionsView = initPositionsView();
}

function createViewWindow() {
    let fieldOpts = {isFixed: true, isDisabled: true};

    return ui.Window("View", "view", "./asset/odin_cesium/camera.svg")(
        ui.RowContainer()(
            ui.CheckBox("fullscreen", toggleFullScreen),
            ui.HorizontalSpacer(1),
            ui.CheckBox("terrain", toggleTerrain, "view.show_terrain"),
            ui.HorizontalSpacer(1),
            ui.Button("⟘", setDownView, 2.5),  // ⇩  ⊾ ⟘
            ui.Button("⌂", setHomeView, 2.5) // ⌂ ⟐ ⨁
          ),
          ui.RowContainer()(
            ui.TextInput("pointer [φ,λ,m]", "view.pointer.latitude", "5rem", fieldOpts),
            ui.TextInput("", "view.pointer.longitude", "6rem", fieldOpts),
            ui.TextInput("", "view.pointer.elevation", "5.5rem", fieldOpts),
            ui.HorizontalSpacer(0.4)
          ),
          ui.RowContainer()(
            ui.TextInput("UTM [N,E,z]", "view.pointer.utmN", "5rem", fieldOpts),
            ui.TextInput("", "view.pointer.utmE", "6rem", fieldOpts),
            ui.TextInput("", "view.pointer.utmZ", "5.5rem", fieldOpts),
            ui.HorizontalSpacer(0.4)
          ),
          ui.RowContainer()(
            ui.TextInput("camera", "view.camera.latitude", "5rem", {changeAction: setViewFromFields, isFixed: true}),
            ui.TextInput("", "view.camera.longitude", "6rem", {changeAction: setViewFromFields, isFixed: true}),
            ui.TextInput("", "view.camera.altitude", "5.5rem", {changeAction: setViewFromFields, isFixed: true}),
            ui.HorizontalSpacer(0.4)
          ),
          ui.RowContainer()(
            ui.Choice("","view.posSet", selectPositionSet),
            ui.Button("save", storePositionSet, 3.5),
            ui.Button("del", removePositionSet, 3.5)
          ),
          ui.List("view.positions", 8, setCameraFromSelection),
          ui.RowContainer()(
            ui.Button("⨀", pickPoint),
            ui.Button("⨁", addPoint),
            ui.Button("⌫", removePoint)
          ),
          ui.Panel("view parameters", false)(
            ui.CheckBox("render on-demand", toggleRequestRenderMode, "view.rm"),
            ui.Slider("frame rate", "view.fr", setFrameRate)
          )
    );
}

function initPositionsView() {
    let view = ui.getList("view.positions");
    if (view) {
        ui.setListItemDisplayColumns(view, ["fit", "header"], [
            { name: "", tip: "show/hide ground point", width: "2rem", attrs: [], map: e => ui.createCheckBox(e.asset, toggleShowPosition) },
            { name: "name", tip: "place name name", width: "6rem", attrs: [], map: e => e.name },
            { name: "lat", tip: "latitude [deg]", width:  "5.5rem", attrs: ["fixed", "alignRight"], map: e => util.formatFloat(e.lat,4)},
            { name: "lon", tip: "longitude [deg]", width:  "6.5rem", attrs: ["fixed", "alignRight"], map: e => util.formatFloat(e.lon,4)},
            { name: "alt", tip: "altitude [m]", width:  "5.5rem", attrs: ["fixed", "alignRight"], map: e => Math.round(e.alt)}
        ]);

        selectedPositionSet = positionSets[0];
        positions = selectedPositionSet.positions;
        ui.setChoiceItems("view.posSet", positionSets, 0);
        ui.setListItems(view, positions);
    }

    return view;
}

function createViewIcon() {
    return ui.Icon("./asset/odin_cesium/camera.svg", (e)=> ui.toggleWindow(e,'view'));
}

//--- time window

function initTimeWindow() {
    createTimeIcon();
    createTimeWindow();
}

function createTimeWindow() {
    return ui.Window("clock", "time", "./asset/odin_cesium/time.svg")(
        ui.Clock("time UTC", "time.utc", "UTC"),
        ui.Clock("time loc", "time.loc",  config.localTimeZone),
        ui.Timer("elapsed", "time.elapsed")
    );
}

function createTimeIcon() {
    return ui.Icon("./asset/odin_cesium/time.svg", (e)=> ui.toggleWindow(e,'time'));
}

//--- layer window

function initLayerWindow() {
    createLayerIcon();
    createLayerWindow();
    layerOrderView = initLayerOrderView();
    layerHierarchyView = initLayerHierarchyView();
}

function createLayerWindow() {
    return ui.Window("module layers", "layer", "./asset/odin_cesium/layers.svg")(
        ui.Panel("module Z-order", true)(
            ui.List("layer.order", 10),
            ui.RowContainer()(
                ui.Button("↑", raiseModuleLayer),
                ui.Button("↓", lowerModuleLayer)
            )
        ),
        ui.Panel("module hierarchy", false)(
            ui.TreeList("layer.hierarchy", 15, 25)
        )
    );
}

function createLayerIcon() {
    return ui.Icon("./asset/odin_cesium/layers.svg", (e)=> ui.toggleWindow(e,'layer'));
}

function initLayerOrderView() {
    let v = ui.getList("layer.order");
    if (v) {
        ui.setListItemDisplayColumns(v, ["fit", "header"], [
            { name: "", width: "2rem", attrs: [], map: e =>  setLayerOrderCb(e) },
            { name: "name", width: "8rem", attrs: [], map: e => e.name },
            { name: "cat", width: "10rem", attrs: [], map: e => e.category}
        ]);
    }
    return v;
}

//--- view position sets

function selectPositionSet(event) {
    let posSet = ui.getSelectedChoiceValue(event);
    if (posSet) {
        selectedPositionSet = posSet;
        positions = selectedPositionSet.positions;
        ui.setListItems(positionsView, positions);
    }
}

function toggleShowPosition(event) {
    let cb = ui.getCheckBox(event.target);
    if (cb) {
        let pos = ui.getListItemOfElement(cb);
        if (pos) {
            if (ui.isCheckBoxSelected(cb)){
                if (!pos.asset) setPositionAsset(pos);
            } else {
                if (pos.asset) clearPositionAsset(pos);
            }
        }
    }
}

function addPoint() {
    let latDeg = Number.parseFloat(ui.getFieldValue("view.camera.latitude"));
    let lonDeg = Number.parseFloat(ui.getFieldValue("view.camera.longitude"));
    let altM = Number.parseFloat(ui.getFieldValue("view.camera.altitude"));

    if (isNaN(latDeg) || isNaN(lonDeg) || isNaN(altM)){
        alert("please enter valid latitude, longitude and altitude");
        return;
    }

    let name = prompt("please enter point name", positions.length.toString());
    if (name) {
        let pt = new Position(name, latDeg, lonDeg, altM);
        positions = util.copyArrayIfSame( selectedPositionSet.positions, positions);
        positions.push(pt);
        ui.setListItems(positionsView, positions);
    }
}

function pickPoint() {
    let btn = ui.getButton("view.pickPos");
    ui.setElementColors( btn, ui.getRootVar("--selected-data-color"), ui.getRootVar("--selection-background"));

    // system prompt blocks DOM manipulation so we need to defer the action
    setTimeout( ()=> {
        let name = prompt("please enter point name and click on map", selectedPositionSet.positions.length);
        if (name) {
            pickSurfacePoint( (cp) => {
                if (cp) {
                    let latDeg = util.toDegrees(cp.latitude);
                    let lonDeg = util.toDegrees(cp.longitude);
                    let altM = ui.getFieldValue("view.camera.altitude");
                    
                    ui.setField("view.pointer.latitude", latDeg);
                    ui.setField("view.pointer.longitude", lonDeg);
                    
                    let pt = new Position(name, latDeg, lonDeg, altM);
                    positions = util.copyArrayIfSame( selectedPositionSet.positions, positions);
                    positions.push(pt);
                    ui.setListItems("view.positions", positions);
                }
                ui.resetElementColors(btn);
            });
        } else {
            ui.resetElementColors(btn);
        }
    }, 100);
}

function namePoint() {
}

function removePoint() {
    let pos = ui.getSelectedListItem(positionsView);
    if (pos) {
        let idx = positions.findIndex( p=> p === pos);
        if (idx >= 0) {
            positions = util.copyArrayIfSame( selectedPositionSet.positions, positions);
            positions.splice(idx, 1);
            ui.setListItems(positionsView, positions);
        }
    }
}

function getPositionSets() {
    let sets = [];
    sets.push( getGlobalPositionSet());
    getLocalPositionSets().forEach( ps=> sets.push(ps));
    return sets;
}

// TODO - we should support multiple gobal position sets
function getGlobalPositionSet() { // from config
    let positions = config.cameraPositions.map( p=> new Position(p.name, p.lat, p.lon, p.alt));
    let pset = new PositionSet("default", positions);

    let initPos = getInitialPosition();
    if (initPos) {
        initPosition = initPos;
        pset.positions.unshift(initPos)
    }

    homePosition = positions.find( p=> p.name === "home");
    if (!homePosition) homePosition = positions[0];

    return pset;
}

function getInitialPosition() {
    let queryString = window.location.search;
    if (queryString.length > 0) {
        let params = new URLSearchParams(queryString);
        let view = params.get("view");
        if (view) {
            let elems = view.split(',');
            if (elems.length > 1) {
                try {
                    for (let i=0; i<elems.length; i++) {
                        elems[i] = parseFloat( elems[i], 10);
                    }
                    if (elems.length == 2) { // no height given
                        elems.push( 150000);
                    } else {
                        if (elems[2] < 10000) { // assume this is in km
                            elems[2] = elems[2] * 1000;
                        }
                    }
                    return new Position( "<initial>", elems[0], elems[1], elems[2]); // name,lat,lon,alt

                } catch (e) {
                    console.log("ignoring invalid initial position spec: ", view);
                }
            }
        }
    }
    return null;
}

function getLocalPositionSets() { // from local storage
    let psets = localStorage.getItem(UI_POSITIONS);
    return psets ? JSON.parse(psets) : [];
}

function filterAssets(k,v) {
    if (k === 'asset') return undefined;
    else return v;
}

function storePositionSet() {
    if (selectedPositionSet) {
        let psName = selectedPositionSet.name;
        if (!psName.startsWith(LOCAL)) psName = LOCAL + psName;

        psName = prompt("please enter name for local poisition set",psName);
        if (psName) {
            if (!psName.startsWith(LOCAL)) psName = LOCAL + psName;

            let newPss = getLocalPositionSets();
            let newPs = new PositionSet(psName, positions);
            let idx = newPss.findIndex(e => e.name === psName);
            if (idx <0 ) {
                newPss.push( newPs);
                idx = newPss.length-1;
            } else {
                newPss[idx] = newPs;
            }

            localStorage.setItem(UI_POSITIONS, JSON.stringify( newPss, filterAssets));

            newPss.unshift(getGlobalPositionSet());
            selectedPositionSet = newPs;
            positions = selectedPositionSet.positions;
            ui.setChoiceItems("view.posSet", newPss, idx+1);
            ui.selectChoiceItem("view.posSet", newPs);
            ui.setListItems(positionsView, positions);
        }
    }
}

function removePositionSet() {
    if (selectedPositionSet) {
        let psName = selectedPositionSet.name;
        if (!psName.startsWith(LOCAL)) {
            alert("denied - cannot remove non-local position sets");
            return;
        }

        let localPs = getLocalPositionSets();
        let idx = localPs.findIndex(e => e.name === psName);
        if (idx >= 0){
            if (confirm("delete position set " + selectedPositionSet.name)) {
                if (localPs.length == 1) {
                    localStorage.removeItem(UI_POSITIONS);
                } else {
                    localPs.splice(idx, 1);
                    localStorage.setItem(UI_POSITIONS, JSON.stringify(localPs));
                }

                let ps = getPositionSets();
                selectedPositionSet = ps[0];
                positions = selectedPositionSet.positions;
                ui.setChoiceItems("view.posSet", ps, 0);
                ui.setListItems(positionsView, positions);
            }
        }
    }
}

function setPositionAsset(pos) {
    let cfg = config;

    let e = new Cesium.Entity({
        id: pos.name,
        position: Cesium.Cartesian3.fromDegrees( pos.lon, pos.lat),
        point: {
            pixelSize: cfg.pointSize,
            color: cfg.color,
            outlineColor: cfg.outlineColor,
            outlineWidth: 1,
            disableDepthTestDistance: Number.NEGATIVE_INFINITY
        },
        label: {
            text: pos.name,
            font: cfg.font,
            fillColor: cfg.outlineColor,
            showBackground: true,
            backgroundColor: cfg.labelBackground,
            //heightReference: Cesium.HeightReference.CLAMP_TO_GROUND,
            horizontalOrigin: Cesium.HorizontalOrigin.LEFT,
            verticalOrigin: Cesium.VerticalOrigin.TOP,
            pixelOffset: new Cesium.Cartesian2( 5, 5)
        }
    });
    pos.asset = e;
    dataSource.entities.add(e);
    requestRender();
}

function clearPositionAsset(pos) {
    if (pos.asset) {
        dataSource.entities.remove(pos.asset);
        pos.asset = undefined;
        requestRender();
    }
}

function initFrameRateSlider() {
    let e = ui.getSlider('view.fr');
    if (e) {
        ui.setSliderRange(e, 0.0, 60, 10, util.f_0);
        ui.setSliderValue(e, targetFrameRate);
    }
}

function setTargetFrameRate(fr) {
    targetFrameRate = fr;
    if (fr > 0) {
        viewer.targetFrameRate = targetFrameRate;
    } else {
        viewer.targetFrameRate = undefined; // whatever the browser default animation rate is
    }
}

export function lowerFrameRateWhile(action, lowFr) {
    viewer.targetFrameRate = lowFr;
    action();
    viewer.targetFrameRate = targetFrameRate;
}

export function lowerFrameRateFor(msec, lowFr) {
    let curFr = viewer.targetFrameRate;
    viewer.targetFrameRate = lowFr;
    setTimeout(() => {
        viewer.targetFrameRate = curFr;
        requestRender();
    }, msec);
}

export function setRequestRenderMode(cond) {
    requestRenderMode = cond;
    viewer.scene.requestRenderMode = cond;
    ui.setCheckBox("view.rm", cond);
}

export function isRequestRenderMode() {
    return requestRenderMode;
}

export function toggleRequestRenderMode() {
    requestRenderMode = !requestRenderMode;
    viewer.scene.requestRenderMode = requestRenderMode;
    ui.setCheckBox("view.rm", requestRenderMode);
}

export function requestRender() {
    if (requestRenderMode && !pendingRenderRequest) {
        pendingRenderRequest = true;
        viewer.scene.requestRender();
    }
}

export function withSampledTerrain(positions, level, action) {
    const promise = Cesium.sampleTerrain(viewer.terrainProvider, level, positions);
    Promise.resolve(promise).then(action);
}

export function withDetailedSampledTerrain(positions, action) {
    const promise = Cesium.sampleTerrainMostDetailed(viewer.terrainProvider, positions);
    Promise.resolve(promise).then(action);
}

export function createScreenSpaceEventHandler() {
    return new Cesium.ScreenSpaceEventHandler(viewer.scene.canvas);
}

export function setCursor(cssCursorSpec) {
    viewer.scene.canvas.style.cursor = cssCursorSpec;
}

export function setDefaultCursor() {
    viewer.scene.canvas.style.cursor = "default";
}

function setCanvasSize() {
    viewer.canvas.width = window.innerWidth;
    viewer.canvas.height = window.innerHeight;
}

export function setDoubleClickHandler (action) {
    let selHandler = new Cesium.ScreenSpaceEventHandler(viewer.scene.canvas);
    selHandler.setInputAction(action, Cesium.ScreenSpaceEventType.LEFT_DOUBLE_CLICK);
}

export function setEntitySelectionHandler(onSelect) {
    let selHandler = new Cesium.ScreenSpaceEventHandler(viewer.scene.canvas);
    selHandler.setInputAction(onSelect, Cesium.ScreenSpaceEventType.LEFT_CLICK);
}

export function addDataSource(dataSrc) {
    viewer.dataSources.add(dataSrc);
}

export function removeDataSource(dataSrc) {
    viewer.dataSources.remove(dataSrc);
}

export function toggleDataSource(dataSrc) {
    if (viewer.dataSources.contains(dataSrc)) {
        viewer.dataSources.remove(dataSrc);
    } else {
        viewer.dataSources.add(dataSrc);
    }
}

export function isDataSourceShowing(dataSrc) {
    return viewer.dataSources.contains(dataSrc);
}

export function addPrimitive(prim) {
    viewer.scene.primitives.add(prim);
}

export function addPrimitives(primitives) {
    let pc = viewer.scene.primitives;
    primitives.forEach( p=> pc.add(p));
    requestRender();
}

export function showPrimitive(prim, show) {
    prim.show = show;
    requestRender();
}
export function showPrimitives(primitives, show) {
    primitives.forEach( p=> p.show = show);
    requestRender();
}

export function removePrimitive(prim) {
    viewer.scene.primitives.remove(prim); // watch out - this destroys prim
}
export function removePrimitives(primitives) {
    let pc = viewer.scene.primitives;
    primitives.forEach( p=> pc.remove(p));
    requestRender();
}


export function clearSelectedEntity() {
    viewer.selectedEntity = null;
}

export function getSelectedEntity() {
    return viewer.selectedEntity;
}

export function setSelectedEntity(e) {
    viewer.selectedEntity = e;
}

export function addEntity(e) {
    viewer.entities.add(e);
}
export function removeEntity(e) {
    viewer.entities.remove(e);
}


//--- websock handler funcs

function handleWsMessages(msgType, msg) {
    switch (msgType) {
        case "camera":
            handleCameraMessage(msg.camera);
        case "clock":
            handleSetClock(msg);
    }
}

function handleCameraMessage(newCamera) {
    cameraSpec = newCamera;
    setHomeView();
}

function handleSetClock(setClock) {
    ui.setClock("time.utc", setClock.time, setClock.timeScale, true);
    ui.setClock("time.loc", setClock.time, setClock.timeScale);
    ui.resetTimer("time.elapsed", setClock.timeScale);
    ui.startTime();
}

function updateCamera() {
    let pos = viewer.camera.positionCartographic;
    let longitudeString = Cesium.Math.toDegrees(pos.longitude).toFixed(4);
    let latitudeString = Cesium.Math.toDegrees(pos.latitude).toFixed(4);

    ui.setField(cameraLat, latitudeString);
    ui.setField(cameraLon, longitudeString);
    ui.setField(cameraAlt, Math.round(pos.height).toString());

    if (isSelectedView) {
        isSelectedView = false;
    } else {
        ui.clearSelectedListItem(positionsView); // we moved away from it
    }

    /*
    if (useEllipsoidTerrain()) {
        switchToEllipsoidTerrain(); // this checks if we already use it
    } else {
        switchToTopoTerrain();
    }
    */

    //saveCamera();
}


//--- mouse event handlers

export function registerMouseMoveHandler(handler) {
    mouseMoveHandlers.push(handler);
}

export function releaseMouseMoveHandler(handler) {
    let idx = mouseMoveHandlers.findIndex(h => h === handler);
    if (idx >= 0) mouseMoveHandlers.splice(idx,1);
}

export function registerMouseClickHandler(handler) {
    mouseClickHandlers.push(handler);
}

export function releaseMouseClickHandler(handler) {
    let idx = mouseClickHandlers.findIndex(h => h === handler);
    if (idx >= 0) mouseClickHandlers.splice(idx,1);
}

export function registerMouseDblClickHandler(handler) {
    mouseDblClickHandlers.push(handler);
}

export function releaseMouseDblClickHandler(handler) {
    let idx = mouseDblClickHandlers.findIndex(h => h === handler);
    if (idx >= 0) mouseDblClickHandlers.splice(idx,1);
}

function handleMouseMove(e) {
    mouseMoveHandlers.forEach( handler=> handler(e));
}

function handleMouseClick(e) {
    mouseClickHandlers.forEach( handler=> handler(e));
}

function handleMouseDblClick(e) {
    mouseDblClickHandlers.forEach( handler=> handler(e));
}

function getCartographicMousePosition(e) {
    var ellipsoid = viewer.scene.globe.ellipsoid;
    var cartesian = viewer.camera.pickEllipsoid(new Cesium.Cartesian3(e.clientX, e.clientY), ellipsoid);
    if (cartesian) {
        return ellipsoid.cartesianToCartographic(cartesian);
    } else {
        return undefined;
    }
}

function getCartesian3MousePosition(e) {
    var ellipsoid = viewer.scene.globe.ellipsoid;
    return viewer.camera.pickEllipsoid(new Cesium.Cartesian3(e.clientX, e.clientY), ellipsoid);
}

var deferredMouseUpdate = undefined;

function updateMouseLocation(e) {
    if (deferredMouseUpdate) clearTimeout(deferredMouseUpdate);
    deferredMouseUpdate = setTimeout( () => {
        let pos = getCartographicMousePosition(e);
        if (pos) {
            let latDeg = Cesium.Math.toDegrees(pos.latitude);
            let lonDeg = Cesium.Math.toDegrees(pos.longitude);

            let longitudeString = lonDeg.toFixed(4);
            let latitudeString = latDeg.toFixed(4);
    
            ui.setField(pointerLat, latitudeString);
            ui.setField(pointerLon, longitudeString);
    
            if (topoTerrainProvider) {
                let a = [pos];
                Cesium.sampleTerrainMostDetailed(topoTerrainProvider, a).then( (a) => {
                    ui.setField(pointerElev, Math.round(a[0].height));
                });
            }

            let utm = util.latLon2Utm(latDeg, lonDeg);
            ui.setField(pointerUtmN, utm.northing);
            ui.setField(pointerUtmE, utm.easting);
            ui.setField(pointerUtmZ, `${utm.utmZone} ${utm.band}`);
        }
    }, 300);
}

//--- user control 

function setViewFromFields() {
    let lat = ui.getFieldValue(cameraLat);
    let lon = ui.getFieldValue(cameraLon);
    let alt = ui.getFieldValue(cameraAlt);

    if (lat && lon && alt) {
        let latDeg = parseFloat(lat);
        let lonDeg = parseFloat(lon);
        let altM = parseFloat(alt);

        // TODO - we should check for valid ranges here
        if (isNaN(latDeg)) { alert("invalid latitude: " + lat); return; }
        if (isNaN(lonDeg)) { alert("invalid longitude: " + lon); return; }
        if (isNaN(altM)) { alert("invalid altitude: " + alt); return; }

        viewer.camera.flyTo({
            destination: Cesium.Cartesian3.fromDegrees(lonDeg, latDeg, altM),
            orientation: centerOrientation
        });
    } else {
        alert("please enter latitude, longitude and altitude");
    }
}

export function saveCamera() {
    let camera = viewer.camera;
    let pos = camera.positionCartographic;

    lastCamera = {
        lat: util.toDegrees(pos.latitude),
        lon: util.toDegrees(pos.longitude),
        alt: pos.height,
        heading: util.toDegrees(camera.heading),
        pitch: util.toDegrees(camera.pitch),
        roll: util.toDegrees(camera.roll)
    };

    // TODO - this should be triggered by a copy-to-clipboard button
    //let spec = `{ lat: ${util.fmax_4.format(lastCamera.lat)}, lon: ${util.fmax_4.format(lastCamera.lon)}, alt: ${Math.round(lastCamera.alt)} }`;
    //navigator.clipboard.writeText(spec);  // this is still experimental in browsers and needs to be enabled explicitly (for each doc?) for security reasons
    //console.log(spec);
}

export function zoomTo(cameraPos) {
    saveCamera();

    viewer.camera.flyTo({
        destination: cameraPos,
        orientation: centerOrientation
    });
}

function setInitialView () {
    let initPos = initPosition ? initPosition : homePosition;
    setCamera( initPos);
}

export function setHomeView() {
    setCamera(homePosition);
}

export function setCamera(camera) {
    saveCamera();

    viewer.selectedEntity = undefined;
    viewer.trackedEntity = undefined;
    viewer.camera.flyTo({
        destination: Cesium.Cartesian3.fromDegrees(camera.lon, camera.lat, camera.alt),
        orientation: centerOrientation
    });
}

function setCameraFromSelection(event){
    let places = ui.getList(event);
    if (places) {
        let cp = ui.getSelectedListItem(places);
        if (cp) {
            setCamera(cp);
            isSelectedView = true;
        }
    }
}

var minCameraHeight = 50000;

export function setDownView() {

    // use the position we are looking at, not the current camera position
    const canvas = viewer.scene.canvas;
    const center = new Cesium.Cartesian2(canvas.clientWidth / 2.0, canvas.clientHeight / 2.0);
    const ellipsoid = viewer.scene.globe.ellipsoid;
    let wc = viewer.camera.pickEllipsoid(center,ellipsoid);
    let pos = Cesium.Cartographic.fromCartesian(wc);

    //let pos = viewer.camera.positionCartographic;
    if (pos.height < minCameraHeight) pos = new Cesium.Cartographic(pos.longitude,pos.latitude,minCameraHeight);

    viewer.trackedEntity = undefined;

    viewer.camera.flyTo({
        destination: Cesium.Cartographic.toCartesian(pos),
        orientation: centerOrientation
    });
}

export function restoreCamera() {
    if (lastCamera) {
        let last = lastCamera;
        saveCamera();
        setCamera(last);
    }
}


export function toggleFullScreen(event) {
    ui.toggleFullScreen();
}

function setFrameRate(event) {
    let v = ui.getSliderValue(event.target);
    setTargetFrameRate(v);
}

//--- module layers


function setLayerOrderCb(le) {
    let cb = ui.createCheckBox(le.show, toggleShowLayer);
    le.layerOrderCb = cb;
    return cb;
}

function initLayerHierarchyView() {
    let v = ui.getList("layer.hierarchy");
    if (v) {

    }
    return v;
}

function toggleShowLayer(event) {
    let cb = ui.getCheckBox(event.target);
    if (cb) {
        let le = ui.getListItemOfElement(cb);
        if (le) le.setVisible(ui.isCheckBoxSelected(cb));
    }
}

// called after all modules have loaded
function initModuleLayerViewData() {
    ui.setListItems(layerOrderView, layerOrder);
}

// called by layer modules during their init - the panel is in the respective module window
export function initLayerPanel(wid, conf, showAction) {
    if (conf && conf.layer) {
        let phe = document.getElementById(wid + ".layer-header");
        if (phe) {
            let le = new LayerEntry(wid,conf.layer,showAction);

            phe.innerText = "layer: " + conf.layer.name.replaceAll('/', '╱'); // │
            let cb = ui.createCheckBox(conf.layer.show, (event) => {
                event.stopPropagation();
                le.setVisible(ui.isCheckBoxSelected(cb));
            });
            ui.positionRight(cb, 0);
            phe.appendChild(cb);
            le.modulePanelCb = cb;

            ui.setLabelText(wid + '.layer-descr', conf.layer.description);

            layerOrder.push(le);
        }
    }
}

export function isLayerShowing(layerPath) {
    let le = layerOrder.find( le=> le.id == layerPath);
    return (le && le.show);
}

function raiseModuleLayer(event){
    let le = ui.getSelectedListItem(layerOrderView);
    console.log("TBD raise layer: " + le);
}

function lowerModuleLayer(event){
    let le = ui.getSelectedListItem(layerOrderView);
    console.log("TBD lower layer: " + le);
}

//--- interactive geo input

export function pickSurfacePoint (callback) {
    let cancel = false;

    function onKeydown(event) {
        if (event.key == "Escape") {
            cancel = true;
            viewer.scene.canvas.click();
        }
    }

    function onClick(event) {
        let p = getCartographicMousePosition(event);
        if (p) { 
            callback( cancel ? null : p)
        }
        setDefaultCursor();
        releaseMouseClickHandler(onClick);
        document.removeEventListener( 'keydown', onKeydown);
    }

    document.addEventListener('keydown', onKeydown);
    setCursor("crosshair");
    registerMouseClickHandler(onClick);
}

// this should normally use a ScreenSpaceEventHandler but that fails for some reason if
// sceneModePicker is enabled (positions are off). This one does not correctly handle terrain but is close enough
export function pickSurfaceRectangle (callback) {
    var asset = undefined;
    var p0 = undefined;
    var rect = undefined;
    let poly = Cesium.Cartesian3.fromDegreesArray([0,0, 0,0, 0,0, 0,0, 0,0]);

    function onMouseMove(event) {
        let p = getCartographicMousePosition(event);
        if (p) {
            rect.west = Math.min( p0.longitude, p.longitude);
            rect.south = Math.min( p0.latitude, p.latitude);
            rect.east = Math.max( p0.longitude, p.longitude);
            rect.north = Math.max( p0.latitude, p.latitude);
            // FIXME - we can do better than to convert back to where we came from. just rotate
            cartesian3ArrayFromRadiansRect(rect, poly);
        }
        requestRender();
    }

    function onClick(event) {
        let p = getCartographicMousePosition(event);
        if (p) { 
            if (!rect) {
                p0 = p;
                rect = new Cesium.Rectangle(p0.longitude, p0.latitude, p0.longitude, p0.latitude);

                asset = new Cesium.Entity({
                    polyline: {
                        positions: new Cesium.CallbackProperty( () => poly, false),
                        clampToGround: true,
                        width: 2,
                        material: Cesium.Color.RED
                    },
                    selectable: false
                });
                viewer.entities.add(asset);
                requestRender();

                registerMouseMoveHandler(onMouseMove);

            } else {
                setDefaultCursor();
                releaseMouseMoveHandler(onMouseMove);
                releaseMouseClickHandler(onClick);
                viewer.entities.remove(asset);

                rect.west = Cesium.Math.toDegrees(rect.west);
                rect.south = Cesium.Math.toDegrees(rect.south);
                rect.east = Cesium.Math.toDegrees(rect.east);
                rect.north = Cesium.Math.toDegrees(rect.north);

                callback(rect);
                requestRender();
            }
        }
    }

    setCursor("crosshair");
    registerMouseClickHandler(onClick);
}

export function pickSurfacePolyline (pointCallback,polylineCallback) {
    let points = []; // Cartesian3 positions
    let polyEntity = undefined;

    function onMouseMove(event) {
        let lastIdx = points.length-1;
        if (lastIdx > 0) {
            let p = getCartesian3MousePosition(event);
            if (p) {
                points[lastIdx] = p;
                requestRender();
            }
        }
    }

    function onClick(event) {
        if (event.detail == 2) { // double click -> terminate
            event.preventDefault(); // Cesium likes to zoom in on double clicks

            setDefaultCursor();
            releaseMouseMoveHandler(onMouseMove);
            releaseMouseClickHandler(onClick);
    
            if (polyEntity) {
                viewer.entities.remove(polyEntity);
            }
    
            if (points.length > 1) {
                //let poly = points.map( p => Cesium.Cartographic.fromCartesian(p));
                let poly = points.map( p=> cartesianToCartographicDegrees(p));
                polylineCallback(poly);
            }

        } else if (event.detail == 1) {
            let p = getCartesian3MousePosition(event);
            if (p) { 
                points.push( p);
                points.push( p); // the moving one

                if (!polyEntity) {
                    polyEntity = new Cesium.Entity( {
                        polyline: {
                            positions: new Cesium.CallbackProperty( () => points, false),
                            clampToGround: true,
                            width: 2,
                            material: Cesium.Color.RED
                        },
                        selectable: false
                    });
                    viewer.entities.add(polyEntity);
                    requestRender();

                    registerMouseMoveHandler(onMouseMove);
                }

                pointCallback( cartesianToCartographicDegrees(p));
            }
        }
    }

    setCursor("crosshair");
    registerMouseClickHandler(onClick);
}

export function cartesianToCartographicDegrees (p) {
    return cartographicToDegrees( Cesium.Cartographic.fromCartesian(p));
}

export function cartographicToDegrees (p) {
    return { latitude: Cesium.Math.toDegrees(p.latitude), longitude: Cesium.Math.toDegrees(p.longitude), height: p.height };
}

export function cartesian3ArrayFromRadiansRect (rect, arr=null) {
    let a = arr ? arr : new Array(5);

    a[0] = Cesium.Cartesian3.fromRadians( rect.west, rect.north);
    a[1] = Cesium.Cartesian3.fromRadians( rect.east, rect.north);
    a[2] = Cesium.Cartesian3.fromRadians( rect.east, rect.south);
    a[3] = Cesium.Cartesian3.fromRadians( rect.west, rect.south);
    a[4] = Cesium.Cartesian3.fromRadians( rect.west, rect.north);

    return a;
}

export function cartesian3ArrayFromDegreesRect (rect, arr=null) {
    let a = arr ? arr : new Array(5);

    a[0] = Cesium.Cartesian3.fromDegrees( rect.west, rect.north);
    a[1] = Cesium.Cartesian3.fromDegrees( rect.east, rect.north);
    a[2] = Cesium.Cartesian3.fromDegrees( rect.east, rect.south);
    a[3] = Cesium.Cartesian3.fromDegrees( rect.west, rect.south);
    a[4] = Cesium.Cartesian3.fromDegrees( rect.west, rect.north);

    return a;
}

export function withinRect(latDeg, lonDeg, degRect) {
    return (lonDeg >= degRect.west) && (lonDeg <= degRect.east) && (latDeg >= degRect.south) && (latDeg <= degRect.north);
}

export function getHprFromQuaternion (qx, qy, qz, w) {
    let q = new Cesium.Quaternion( qx, qy, qz, w);
    return Cesium.HeadingPitchRoll.fromQuaternion(q);
}

export function getEnuRotFromQuaternion (qx, qy, qz, w) {
    let q = new Cesium.Quaternion( qx, qy, qz, w);
    let qRot = Cesium.Quaternion.inverse(q, new Cesium.Quaternion());
    return Cesium.Matrix3.fromQuaternion( qRot);
}

function setCesiumContainerVisibility (isVisible) {
    document.getElementById("cesiumContainer").style.visibility = isVisible;
}

// executed after all modules have been loaded and initialized
export function postInitialize() {
    initModuleLayerViewData();    
    terrainProviderPromise = getTerrainProviderPromise();
    terrainProviderPromise.then( (tp) => { 
        console.log("topoTerrainProvider set: ", tp);
        topoTerrainProvider = tp;
        console.log("topographic terrain loaded");

        if (config.showTerrain) {
            console.log("enabling terrain display");
            ui.setCheckBox( "view.show_terrain", true);
            switchToTopoTerrain();
        }
    });

    const credit = new Cesium.Credit('<a href="https://openstreetmap.org/" target="_blank">OpenStreetMap</a>');
    viewer.creditDisplay.addStaticCredit(credit);

    setCesiumContainerVisibility(true);

    console.log("odin_cesium.postInitialize complete.");
}
