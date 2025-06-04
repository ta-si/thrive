# World Modeling
Using noise gen and lookup tables to determine core aspects of a region
to determine various global factors such as biomes, geologcy, flora/fauna.

### Factors
Primary: Rainfall, Temperature
Secondary: Prevailing Wind, Rain Shadow, Elevation, Latitude
Note: Coriolis effect on prevailing winds and oceanic currents

### Categorization
#### Biome
```
100" (annual rainfall)
↑           │            │     Temperate     │   Tropical
↑           │            │     Rainforest    │   Rainforest
↑           │   Boreal   ├───────────────────┤
↑           │   Forest   │     Temperate     ├──────────────────
↑           │            │     Forest        │   Tropical
↑           │            │                   │   Seasonal
↑           ├────────────┼───────────────────┤   Forest
↑           │ Grasslands │     Scrub         ├──────────────────
↑           │            │                   │   Tropical Scrub
↑           ├────────────┼───────────────────┼──────────────────
↑           │   Tundra   │     Plains        │   Tropical 
↑           │            │                   │   Savanna
↑           ├────────────┴───────────────────┴──────────────────
0"  Artic   │                     Desert
  -80*F   →   →   →   →   →   →   →   →   →   →   →   →   120*F
```

^ I could use a tiny texture or mesh to encode this table, allowing for non-rectangular boundaries.

Each plant has a bell curve representing a range of favorability per:
- Temperature, Rainfall, Soil Type?


## Life
### Prairie
[Shortgrass Prairie](https://en.wikipedia.org/wiki/Shortgrass_prairie)
- Region
    - rainfall: 30-35 in
    - soil: 
- Flora
    - Grass
        - Blue Grama
        - Buffalograss
        - Geasegrass
        - Sideoats Grama
    - Plants
        - Soadweed Yucca
        - Plains Prickly Pear
    - Shrubs
    - Trees
- Fauna
    - Birds
        - Sparrow
        - Sandhill Crane
        - Scaled Quail
        - Swainson's Hawk
        - Burrowing Owl
    - Reptiles
        - Roundtail Horned Lizard
        - Garter Snake
    - Mammals
        - Buffalo / Bison / Cattle
        - Pronghorn
        - Prairie Dog

[Tallgrass Prairie](https://en.wikipedia.org/wiki/Tallgrass_prairie)
- Flora
    - Indian grass
    - Switch grass
    - Big Bluestem
- Fauna
    - Pheasent
    - Quail




#### Soil
- Organic: SOM (soil organic matter)
- Inorganic: Sand, Silt, Clay (drainage property?)
- Chemical: Carbon, Nitrogen, Sulfur, Phosphorus, ... others?
- Other factors: acidity?

#### Geological
I am not interested in modeling Techtonic plates, just their affects after millions of years.
I'll generate a rough geologic provinces map most likely, which influences rock/cliff type/color.
Types: Shield, Platform, Orogen, B asin, Large Igneous, Extended Crust?

#### Rock Classification
- Sedimentary (compressed soil)
- Metamorphic (heat/pressure changing Sedimentary)
- Igneous (cooled magma/lava)

### Resources
- https://procworld.blogspot.com/2016/07/geometry-is-destiny-part-2.html
- https://oceantracks.org/library/oceanographic-factors/ocean-currents
- https://www.antarcticglaciers.org/glaciers-and-climate/ocean-circulation/