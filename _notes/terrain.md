# Terrain System

- spawn tiles around tile loaders, flag tiles outside range for removal
  - noiz for now, later: splines, meshes, textures, painting, ...
- create a base mesh, utilize heightmaps, normalmaps per tile
- SplatMat (terrain material), using assets/terrain/splatmats/...
  - a conf.ron controls the gpu struct settings, maps textures
  - default filename suffix allows for auto-binding
- tiles get a splatmap (RGBA 16bit, 1st byte splatmat id, 2nd byte blendweight)
  - height-based blending
  - splatmat textures bound as arrays, not all splatmats need every PBR texture
