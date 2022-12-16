# 2022-01-28

Test location (d1_trainstation_01):
```
pos: guVector {
    x: -4295.0,
    y: -2543.0,
    z: 140.0,
},
yaw: 3.6915,
pitch: 0.0155,
```

## Baseline

P0 CLOCKS: 93,000
P1 TX_MEMSTALL: 16,820
P1 VC_ALL_STALLS: 1,826,000
P1 CLOCKS: 851,320

## Texture cache for TEXMAP0-3 only

P0 CLOCKS: 93,000
P1 TX_MEMSTALL: 13,190
P1 VC_ALL_STALLS: 1,826,000
P1 CLOCKS: 851,800

## Texture cache for TEXMAP0-3 only + bilinear

P0 CLOCKS: 93,000
P1 TX_MEMSTALL: 10,470
P1 VC_ALL_STALLS: 1,825,000
P1 CLOCKS: 851,800

## Global shared texture cache

P0 CLOCKS: 167,000
P1 TX_MEMSTALL: 196,600
P1 VC_ALL_STALLS: 2,896,000
P1 CLOCKS: 851,300

## December 2022 improvements (no envmaps)

P0 CLOCKS: 49,920
P1 TX_MEMSTALL: 20,570
P1 VC_ALL_STALLS: 1,678,000
P1 CLOCKS: 579,200
