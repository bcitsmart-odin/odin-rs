// The version of this with the private cesium access token should be in ODIN_HOME/assets/odin_cesium
// Storing this version here so it is repo, if you want to make changes make sure to recopy this over to version in assets folder

// this has to be set *before* calling any Cesium functions
Cesium.Ion.defaultAccessToken = null; // replace with your Cesium Ion access token from https://ion.cesium.com/tokens?page=1

export const config = {
  terrainProvider: Cesium.createWorldTerrainAsync(),
  showTerrain: true,
  requestRenderMode: true,
  targetFrameRate: -1,
  cameraPositions: [
      { name: "Vancouver", lat: 49.2827, lon: -123.1207, alt: 50000 },
      { name: "North Shore Mountains", lat: 49.3965, lon: -123.0800, alt: 25000 },
      { name: "Richmond", lat: 49.1666, lon: -123.1336, alt: 30000 },
      { name: "Burnaby", lat: 49.2488, lon: -122.9805, alt: 20000 },  
      { name: "Metro Vancouver Area", lat: 49.3, lon: -123.0, alt: 120000 },  // Wide view
      { name: "Space View", lat: 49.2827, lon: -123.1207, alt: 5000000 }  // High altitude view
  ],
  localTimeZone: 'America/Vancouver',
  color: Cesium.Color.fromCssColorString('red'), 
  outlineColor: Cesium.Color.fromCssColorString('yellow'),
  font: '16px sans-serif',
  labelBackground: Cesium.Color.fromCssColorString('#00000060'),
  pointSize: 3,
};
