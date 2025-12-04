# ZM9K-BLE R5.3 Board

### Power
#### Powering nRF52840
- Normal Voltage Mode (Min: 1.7v, Nominal: 3.0v, Max: 3.6v)
  - Powered by External 3v3 Regulator connected to `VDD` & `VDDH`
  - `DC/DC` in `REG1` is Off
  - `REG0` is disabled (Only enabled in High Voltage Mode)
    - Therefore `DC/DC` in `REG0` is Off
  - No External Power LC filter components are on board for either DC/DC regulator
    - ⚠️ So do not enable `DC/DC` for either regulator (`REG1` or `REG0`)
      - Doing so will inhibit device operation, including debug access, until an LC filter is connected or `DC/DC` is disabled in both regulators

### Battery
#### Level Reading
- External Divider Circuit
  - Top:
    - (+)Postive End of Li-Ion Battery
    - 400kΩ resistor
  - Middle:
    - Connected to Pin `P0_04` / `AIN2`
  - Bottom:
    - 1MΩ resistor
    - GND


### Inputs
#### Keys
Direct Pin Matrix Configuration (Columns are left to right, Rows are top to bottom)
- **Row 0:**
  - Column 0: `P0_13` or `0,0`
  - Column 1: `P0_15` or `0,1`
  - Column 2: `P0_17` or `0,2`

- **Row 1:**
  - Column 0: `P0_24` or `1,0`
  - Column 1: `P0_22` or `1,1`
  - Column 2: `P0_20` or `1,2`

- **Row 2:**
  - Column 0: `P1_06` or `2,0`
  - Column 1: `P1_04`   or `2,1`
  - Column 2: `P1_00`   or `2,2`

- **Row 3:**
  - Column 0: `P0_11` or `3,0` (also shared with Encoder #1 Click/Press)


#### Encoder #1
- Pin A
  - `P1_09`
- Pin B
  - `P0_12`
- Click/Press
  - `P0_11`


### SK6812MINI-E / WS2812B 
#### Info
- Number of LEDs:
  - 9
- First is top Left
- Goes order from left to right, top to bottom
- Last is bottom Right

#### Data
First SK6812MINI-E / WS2812B is connected to Pin `P0_26`
- Which is running SPI at 4MHz with some bit pattern manipulation to match the WS2812B's clock requirements

#### Power Control
Non Inverting Control PMOS High Side Switch Circuit

- Pin `P0_25`
  - Where
    - High -> LED Power is on
    - Low -> LED Power is off