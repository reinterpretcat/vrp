# Statistic

A statistic entity represents total statistic for the whole solution or one tour. It has the following structure:

* **cost**: a cost in abstract units
* **distance**: a total distance in distance units
* **duration**: a total duration in duration units
* **times**: a duration split into specific groups:
    * **driving**: a total driving duration
    * **serving**: a total serving jobs duration
    * **waiting**: a total waiting time for time windows
    * **break**: a total break duration


 A solution statistic example:

 ```json
 {{#include ../../../../../examples/json-pragmatic/data/simple.basic.solution.json:2:11}}
 ```