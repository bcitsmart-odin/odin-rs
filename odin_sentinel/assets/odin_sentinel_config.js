export const config = {
    layer: {
      name: "/fire/detection/Sentinel",
      description: "stationary Sentinel fire sensors",
      show: true,
    },
    color: Cesium.Color.fromCssColorString('chartreuse'),
    alertColor: Cesium.Color.fromCssColorString('deeppink'),
    labelFont: '16px sans-serif',
    labelBackground: Cesium.Color.fromCssColorString('black'),
    labelOffset: new Cesium.Cartesian2( 8, 0),
    labelDC: new Cesium.DistanceDisplayCondition( 0, 200000),
    pointSize: 5,
    pointOutlineColor: Cesium.Color.fromCssColorString('black'),
    pointOutlineWidth: 1,
    pointDC: new Cesium.DistanceDisplayCondition( 120000, Number.MAX_VALUE),
    infoFont: '14px monospace',
    infoOffset:  new Cesium.Cartesian2( 8, 16),
    infoDC: new Cesium.DistanceDisplayCondition( 0, 40000),
    billboardDC: new Cesium.DistanceDisplayCondition( 0, 120000),
    imageWidth: 400,
    maxHistory: 10,
    zoomHeight: 20000
  };