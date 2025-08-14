# TODO

Picking / Gizmo
    - picking any entity, sub-set of entities
    - system running over all interactions, per entity callback for an interaction
    - selected entity transform gizmo: https://github.com/urholaukkarinen/transform-gizmo
    - multi-selection + box selection
    - other casting? (physics may be needed for this)

Physics (Avian)
    - colliders
    - casting
    - collisions
    - constraints
    - lib: https://github.com/Jondolf/avian

Images
    - create an image (render it to quad)
    - get pixel value at UV position
    - update image realtime
    - up/down scaling an image to a desired resoltion
    - creating a texture array from textures (same size)
    - blitting from gpu?

Noise
    - lib: https://github.com/johanhelsing/noisy_bevy 
    - octaves, seeds, piping
    - controlling in real time
    - sampling, sampler, preset inputs
    - to image

Curves (like Unity AnimationCurve)
    - stupid lightweight and fast
    - lib: https://github.com/villor/bevy_lookup_curve

Splines
    - points, transform gizmos
    - sampling
        - in/out
        - closest point
        - T to point, point to T (along line)
    - editing
        - add/remove/insert points, whole splines
        - grouping? group sampling?
    - gpu sampling?

Meshes
    - custom mesh

Shaders
    - custom shader
    - compute shader
    - custom pipeline

Others
    Input Mgmt      https://github.com/Leafwing-Studios/leafwing-input-manager
    Terrain
    Prefabs
    Save Files
    Networking      https://github.com/cBournhonesque/lightyear
    Particles       https://github.com/djeedai/bevy_hanabi
    Mesh Ops 
    