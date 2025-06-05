# Terrain System

## Goal
I'm looking to create a plugin which:
- chunks/tiles a large open world
- efficiently renders tiles
- custom shader
    - one draw-call
    - per tile:
        - struct defining choord, texture arrays indices
        - one heightmap (in texture array)
        - one splat map (in texture array)
        - one normal map (in texture array)
    - Splatmaps
        - format is RGBAUint16
            - first byte is TerrainMaterial index
            - second byte is TerrainMaterial blend weight
        - material index is for a list of 'TerrainMaterial' structs:
            - contain indexes into PBR texture arrays:
                - base_color, normal, metallic/roughness, AO, emission, flowmap, etc
            - material uv translation, rotation, scale
            - includes default PBR values if a texture index set to a dedicated 'none' index
            - (future) settings to prevent visual tiling
        - Splatmap 
        - height-based blending with a simple transition curve (non-sharp transitions between materials)