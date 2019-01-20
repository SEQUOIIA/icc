# ICC -  Internet connectivity checker
Master - [![Build Status](https://travis-ci.com/SEQUOIIA/icc.svg?branch=master)](https://travis-ci.com/SEQUOIIA/icc)

ICC pings external IP addresses, and if there is not multiple responses within a customizable time period, it will determine that as WAN being down, and save a timestamp of the current time. As soon as the pinger receive responses again, it will once again save a timestamp of the current time. These two timestamps will be saved together and noted down as "internet downtime". 

  - [ ] Downtime will be stored locally in a file
  - [ ] A web interface that shows uptime/downtime of internet
    - [ ] Graphs
    - [ ] List showing downtime periods
  - [x] Cross-platform, supports Linux, Windows and OSX.

The [PingUtility](https://github.com/SEQUOIIA/icc/blob/master/icc-bin/src/ping/mod.rs) struct is loosely(Almost 1 to 1, with a few changes here and there to accommodate the needs of this project) based on [fastping-rs](https://github.com/bparli/fastping-rs) by [bparli](https://github.com/bparli)