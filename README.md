[![Build Status](https://travis-ci.org/oniproject/oni.svg?branch=master)](https://travis-ci.org/oniproject/oni)
[![Latest version](https://img.shields.io/crates/v/oni.svg)](https://crates.io/crates/oni)
[![Documentation](https://img.shields.io/badge/docs-master-blue.svg)](https://oniproject.github.io/oni/oni)
[![License](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)](COPYNG)

## TODO

- Remove dependency on `serde`.
- Rewrite `oni_trace`.
- Rewrite `simulator`.
- Improve API.
- Write more documentation.
- Write more examples.
- Optimize ChaCha20 and Poly1305.
- More crypto tests.
- Support MTU less than 1200 bytes.
- Peer-to-peer support.
- Semi-reliable message delivery.

## References

- How do multiplayer games sync their state?
[en:
[1](https://www.cakesolutions.net/teamblogs/how-does-multiplayer-game-sync-their-state-part-1),
[2](https://www.cakesolutions.net/teamblogs/how-does-multiplayer-game-sync-their-state-part-2)]
[ru: [1, 2](https://habr.com/post/328702/)]
- Fast-Paced Multiplayer
[en:
[1](http://www.gabrielgambetta.com/client-server-game-architecture.html),
[2](http://www.gabrielgambetta.com/client-side-prediction-server-reconciliation.html),
[3](http://www.gabrielgambetta.com/entity-interpolation.html),
[4](http://www.gabrielgambetta.com/lag-compensation.html)]
[ru:
[1, 2](https://habr.com/post/302394/),
[3](https://habr.com/post/302834/),
[4](https://habr.com/post/303006/)]
[[Demo](http://www.gabrielgambetta.com/client-side-prediction-live-demo.html)]
- [Glenn Fiedler's Game Development Articles](https://gafferongames.com/)
- [ValveSoftware/GameNetworkingSockets](https://github.com/ValveSoftware/GameNetworkingSockets/blob/master/src/steamnetworkingsockets/clientlib/SNP_WIRE_FORMAT.md)
- [0fps.net/distributed-systems](https://0fps.net/category/programming/distributed-systems/) [[Demo](http://mikolalysenko.github.io/local-perception-filter-demo/)]
- [Development and Deployment of Multiplayer Online Games: from social games to MMOFPS, with stock exchanges in between](http://ithare.com/contents-of-development-and-deployment-of-massively-multiplayer-games-from-social-games-to-mmofps-with-stock-exchanges-in-between/)
- [On Robust Estimation of Network-Wide Packet Loss in 3G Cellular Networks](https://ieeexplore.ieee.org/document/5360721)
