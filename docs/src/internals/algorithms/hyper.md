# Hyper-heuristic

By default, the solver tries to use the dynamic hyper-heuristic which uses [Markov Decision Process](https://en.wikipedia.org/wiki/Markov_decision_process)
mechanism to choose one of pre-defined meta-heuristics on each solution refinement step.


```plantuml
@startuml

[*] --> BestKnown
[*] --> Diverse

state m1 <<choice>>
note left of m1: select one of meta-heuristics

state m2 <<choice>>
note right of m2: select one of meta-heuristics

BestKnown: best known solution used
Diverse: diverse solution used


BestMajorImprovement: reward: +1000
BestMinorImprovement: reward: +100
DiverseImprovement: reward: +10
Stagnated: reward: -1

BestKnown --> m1
Diverse --> m2



m1 --> BestMajorImprovement
m1 --> BestMinorImprovement
m1 --> Stagnated


m2 --> BestMajorImprovement
m2 --> BestMinorImprovement
m2 --> DiverseImprovement
m2 --> Stagnated



BestMajorImprovement --> [*]
BestMinorImprovement --> [*]
DiverseImprovement --> [*]
Stagnated --> [*]

@enduml
```