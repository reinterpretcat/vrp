use crate::checker::solve_and_check;
use crate::json::problem::*;
use crate::json::Location;

#[test]
#[ignore]
fn can_properly_handle_load_without_capacity_violation() {
    let problem = Problem {
        id: "generated_problem_with_reloads".to_owned(),
        plan: Plan {
            jobs: vec![
                JobVariant::Single(Job {
                    id: "eed97002-4b06-4ecf-8bdb-c98cfe086f84".to_owned(),
                    places: JobPlaces {
                        pickup: None,
                        delivery: Some(JobPlace {
                            times: Some(vec![
                                vec!["2020-07-04T09:00:00Z".to_owned(), "2020-07-04T13:00:00Z".to_owned()],
                                vec!["2020-07-04T14:00:00Z".to_owned(), "2020-07-04T18:00:00Z".to_owned()],
                            ]),
                            location: Location { lat: 51.0, lng: 0.0 },
                            duration: 12.0,
                            tag: None,
                        }),
                    },
                    demand: vec![2],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "8c4efb73-a612-40d7-a663-a910c5c7a9df".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![
                                vec!["2020-07-04T09:00:00Z".to_owned(), "2020-07-04T11:00:00Z".to_owned()],
                                vec!["2020-07-04T14:00:00Z".to_owned(), "2020-07-04T16:00:00Z".to_owned()],
                            ]),
                            location: Location { lat: 72.0, lng: 0.0 },
                            duration: 17.0,
                            tag: None,
                        }),
                        delivery: None,
                    },
                    demand: vec![1],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "3d44a2d2-10b8-4c5b-b7a1-83a95fc52b12".to_owned(),
                    places: JobPlaces {
                        pickup: None,
                        delivery: Some(JobPlace {
                            times: Some(vec![
                                vec!["2020-07-04T09:00:00Z".to_owned(), "2020-07-04T11:00:00Z".to_owned()],
                                vec!["2020-07-04T14:00:00Z".to_owned(), "2020-07-04T16:00:00Z".to_owned()],
                            ]),
                            location: Location { lat: 61.0, lng: 0.0 },
                            duration: 10.0,
                            tag: None,
                        }),
                    },
                    demand: vec![2],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "1733edde-b9f2-43e8-90e4-88c3cf56df1c".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![
                                vec!["2020-07-04T09:00:00Z".to_owned(), "2020-07-04T11:00:00Z".to_owned()],
                                vec!["2020-07-04T14:00:00Z".to_owned(), "2020-07-04T18:00:00Z".to_owned()],
                            ]),
                            location: Location { lat: 80.0, lng: 0.0 },
                            duration: 10.0,
                            tag: None,
                        }),
                        delivery: None,
                    },
                    demand: vec![3],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "9c354c0f-80fe-40ec-8565-7e5fc900ac93".to_owned(),
                    places: JobPlaces {
                        pickup: None,
                        delivery: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T09:00:00Z".to_owned(),
                                "2020-07-04T11:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 47.0, lng: 0.0 },
                            duration: 11.0,
                            tag: None,
                        }),
                    },
                    demand: vec![4],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "f9e6d146-f1cc-437e-ba13-4468ed499323".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T09:00:00Z".to_owned(),
                                "2020-07-04T13:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 87.0, lng: 0.0 },
                            duration: 15.0,
                            tag: None,
                        }),
                        delivery: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T09:00:00Z".to_owned(),
                                "2020-07-04T13:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 65.0, lng: 0.0 },
                            duration: 13.0,
                            tag: None,
                        }),
                    },
                    demand: vec![3],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "835089b8-e2ea-4330-aca0-02cce540e33d".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![
                                vec!["2020-07-04T09:00:00Z".to_owned(), "2020-07-04T13:00:00Z".to_owned()],
                                vec!["2020-07-04T14:00:00Z".to_owned(), "2020-07-04T16:00:00Z".to_owned()],
                            ]),
                            location: Location { lat: 38.0, lng: 0.0 },
                            duration: 12.0,
                            tag: None,
                        }),
                        delivery: None,
                    },
                    demand: vec![2],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "d6f84436-5b7d-45e2-9290-257bbe1b8516".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T18:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 42.0, lng: 0.0 },
                            duration: 13.0,
                            tag: None,
                        }),
                        delivery: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T16:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 15.0, lng: 0.0 },
                            duration: 17.0,
                            tag: None,
                        }),
                    },
                    demand: vec![1],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "2e150e5f-f661-48b9-ac8e-659ffc8e5ff6".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T18:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 22.0, lng: 0.0 },
                            duration: 15.0,
                            tag: None,
                        }),
                        delivery: None,
                    },
                    demand: vec![3],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "424a04ca-ede2-4b70-956e-ec3d3d7968c9".to_owned(),
                    places: JobPlaces {
                        pickup: None,
                        delivery: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T09:00:00Z".to_owned(),
                                "2020-07-04T11:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 45.0, lng: 0.0 },
                            duration: 18.0,
                            tag: None,
                        }),
                    },
                    demand: vec![3],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "5c7771b5-773a-43b6-b29f-3b2a3e1ded12".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T18:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 41.0, lng: 0.0 },
                            duration: 13.0,
                            tag: None,
                        }),
                        delivery: None,
                    },
                    demand: vec![2],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "853a6af5-88ba-44b5-8814-4c5a95d007b7".to_owned(),
                    places: JobPlaces {
                        pickup: None,
                        delivery: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T16:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 87.0, lng: 0.0 },
                            duration: 15.0,
                            tag: None,
                        }),
                    },
                    demand: vec![2],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "c0112d66-0482-459c-92bf-6352c38834ce".to_owned(),
                    places: JobPlaces {
                        pickup: None,
                        delivery: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T18:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 70.0, lng: 0.0 },
                            duration: 17.0,
                            tag: None,
                        }),
                    },
                    demand: vec![3],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "4c52480d-02da-4009-951e-2e31f8799857".to_owned(),
                    places: JobPlaces {
                        pickup: None,
                        delivery: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T16:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 34.0, lng: 0.0 },
                            duration: 10.0,
                            tag: None,
                        }),
                    },
                    demand: vec![1],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "6efc3956-48f1-459d-831b-dfd48776e5db".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T09:00:00Z".to_owned(),
                                "2020-07-04T11:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 30.0, lng: 0.0 },
                            duration: 14.0,
                            tag: None,
                        }),
                        delivery: None,
                    },
                    demand: vec![4],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "deead2df-a6b4-4609-816f-64208825c603".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T09:00:00Z".to_owned(),
                                "2020-07-04T11:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 32.0, lng: 0.0 },
                            duration: 19.0,
                            tag: None,
                        }),
                        delivery: None,
                    },
                    demand: vec![2],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "df409c04-a827-46cb-9091-55fa0c9dfe38".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T16:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 52.0, lng: 0.0 },
                            duration: 13.0,
                            tag: None,
                        }),
                        delivery: None,
                    },
                    demand: vec![2],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "634ac0c5-c81e-4f01-89aa-e8f8f6617efb".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T16:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 90.0, lng: 0.0 },
                            duration: 16.0,
                            tag: None,
                        }),
                        delivery: None,
                    },
                    demand: vec![3],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "f39945b4-c0be-41cd-a186-58e865e875d6".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T09:00:00Z".to_owned(),
                                "2020-07-04T11:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 39.0, lng: 0.0 },
                            duration: 17.0,
                            tag: None,
                        }),
                        delivery: None,
                    },
                    demand: vec![2],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "d619341b-98b5-48bb-9e37-9cda5def6cde".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![
                                vec!["2020-07-04T09:00:00Z".to_owned(), "2020-07-04T11:00:00Z".to_owned()],
                                vec!["2020-07-04T14:00:00Z".to_owned(), "2020-07-04T16:00:00Z".to_owned()],
                            ]),
                            location: Location { lat: 89.0, lng: 0.0 },
                            duration: 15.0,
                            tag: None,
                        }),
                        delivery: Some(JobPlace {
                            times: Some(vec![
                                vec!["2020-07-04T09:00:00Z".to_owned(), "2020-07-04T11:00:00Z".to_owned()],
                                vec!["2020-07-04T14:00:00Z".to_owned(), "2020-07-04T16:00:00Z".to_owned()],
                            ]),
                            location: Location { lat: 25.0, lng: 0.0 },
                            duration: 12.0,
                            tag: None,
                        }),
                    },
                    demand: vec![3],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "7b9ae460-6e75-4527-a637-e8933ac74a51".to_owned(),
                    places: JobPlaces {
                        pickup: None,
                        delivery: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T09:00:00Z".to_owned(),
                                "2020-07-04T13:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 18.0, lng: 0.0 },
                            duration: 18.0,
                            tag: None,
                        }),
                    },
                    demand: vec![2],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "487ba839-282b-42cb-944b-4a0320266f85".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![
                                vec!["2020-07-04T09:00:00Z".to_owned(), "2020-07-04T11:00:00Z".to_owned()],
                                vec!["2020-07-04T14:00:00Z".to_owned(), "2020-07-04T16:00:00Z".to_owned()],
                            ]),
                            location: Location { lat: 54.0, lng: 0.0 },
                            duration: 16.0,
                            tag: None,
                        }),
                        delivery: None,
                    },
                    demand: vec![2],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "29902df7-2b45-477a-afad-9daafd022a21".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![
                                vec!["2020-07-04T09:00:00Z".to_owned(), "2020-07-04T13:00:00Z".to_owned()],
                                vec!["2020-07-04T14:00:00Z".to_owned(), "2020-07-04T18:00:00Z".to_owned()],
                            ]),
                            location: Location { lat: 93.0, lng: 0.0 },
                            duration: 16.0,
                            tag: None,
                        }),
                        delivery: None,
                    },
                    demand: vec![3],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "b3e9beff-dd6f-4f6b-a25d-6b38beddc0bc".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T09:00:00Z".to_owned(),
                                "2020-07-04T11:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 98.0, lng: 0.0 },
                            duration: 18.0,
                            tag: None,
                        }),
                        delivery: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T09:00:00Z".to_owned(),
                                "2020-07-04T13:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 53.0, lng: 0.0 },
                            duration: 17.0,
                            tag: None,
                        }),
                    },
                    demand: vec![3],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "64c38843-c0f0-4673-a598-bec7ca10515d".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T16:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 1.0, lng: 0.0 },
                            duration: 10.0,
                            tag: None,
                        }),
                        delivery: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T18:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 46.0, lng: 0.0 },
                            duration: 17.0,
                            tag: None,
                        }),
                    },
                    demand: vec![1],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "72e22456-2d8d-4e81-b468-74bf251c083c".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T09:00:00Z".to_owned(),
                                "2020-07-04T13:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 23.0, lng: 0.0 },
                            duration: 15.0,
                            tag: None,
                        }),
                        delivery: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T09:00:00Z".to_owned(),
                                "2020-07-04T11:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 78.0, lng: 0.0 },
                            duration: 17.0,
                            tag: None,
                        }),
                    },
                    demand: vec![1],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "4a482123-1b24-4517-8586-f952f1646cc4".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T16:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 97.0, lng: 0.0 },
                            duration: 13.0,
                            tag: None,
                        }),
                        delivery: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T18:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 22.0, lng: 0.0 },
                            duration: 19.0,
                            tag: None,
                        }),
                    },
                    demand: vec![3],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "8742f06c-8759-49f2-8f46-20ed2c5076a1".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![
                                vec!["2020-07-04T09:00:00Z".to_owned(), "2020-07-04T11:00:00Z".to_owned()],
                                vec!["2020-07-04T14:00:00Z".to_owned(), "2020-07-04T16:00:00Z".to_owned()],
                            ]),
                            location: Location { lat: 50.0, lng: 0.0 },
                            duration: 10.0,
                            tag: None,
                        }),
                        delivery: None,
                    },
                    demand: vec![4],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "3abe9576-1a75-4f9e-989f-9c432e58f098".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T09:00:00Z".to_owned(),
                                "2020-07-04T13:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 26.0, lng: 0.0 },
                            duration: 15.0,
                            tag: None,
                        }),
                        delivery: None,
                    },
                    demand: vec![2],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "110fda14-939b-4040-9cc0-8a1c78c8e1b4".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T16:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 29.0, lng: 0.0 },
                            duration: 12.0,
                            tag: None,
                        }),
                        delivery: None,
                    },
                    demand: vec![4],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "335b215e-b9a5-4a66-b98a-7b600cecbda5".to_owned(),
                    places: JobPlaces {
                        pickup: None,
                        delivery: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T18:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 71.0, lng: 0.0 },
                            duration: 13.0,
                            tag: None,
                        }),
                    },
                    demand: vec![2],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "4449db0f-e3ef-46d9-a4d6-06e0176df26f".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![
                                vec!["2020-07-04T09:00:00Z".to_owned(), "2020-07-04T11:00:00Z".to_owned()],
                                vec!["2020-07-04T14:00:00Z".to_owned(), "2020-07-04T16:00:00Z".to_owned()],
                            ]),
                            location: Location { lat: 55.0, lng: 0.0 },
                            duration: 17.0,
                            tag: None,
                        }),
                        delivery: Some(JobPlace {
                            times: Some(vec![
                                vec!["2020-07-04T09:00:00Z".to_owned(), "2020-07-04T13:00:00Z".to_owned()],
                                vec!["2020-07-04T14:00:00Z".to_owned(), "2020-07-04T18:00:00Z".to_owned()],
                            ]),
                            location: Location { lat: 88.0, lng: 0.0 },
                            duration: 10.0,
                            tag: None,
                        }),
                    },
                    demand: vec![2],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "d8febdf1-43a2-4daa-88be-214f792a83b6".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T18:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 11.0, lng: 0.0 },
                            duration: 18.0,
                            tag: None,
                        }),
                        delivery: Some(JobPlace {
                            times: Some(vec![
                                vec!["2020-07-04T09:00:00Z".to_owned(), "2020-07-04T13:00:00Z".to_owned()],
                                vec!["2020-07-04T14:00:00Z".to_owned(), "2020-07-04T18:00:00Z".to_owned()],
                            ]),
                            location: Location { lat: 87.0, lng: 0.0 },
                            duration: 10.0,
                            tag: None,
                        }),
                    },
                    demand: vec![3],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "1354398a-02e7-439e-a832-424ff57aef68".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T09:00:00Z".to_owned(),
                                "2020-07-04T11:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 91.0, lng: 0.0 },
                            duration: 15.0,
                            tag: None,
                        }),
                        delivery: Some(JobPlace {
                            times: Some(vec![
                                vec!["2020-07-04T09:00:00Z".to_owned(), "2020-07-04T11:00:00Z".to_owned()],
                                vec!["2020-07-04T14:00:00Z".to_owned(), "2020-07-04T18:00:00Z".to_owned()],
                            ]),
                            location: Location { lat: 53.0, lng: 0.0 },
                            duration: 19.0,
                            tag: None,
                        }),
                    },
                    demand: vec![2],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "7a695637-3407-49d6-9289-3189e11f011c".to_owned(),
                    places: JobPlaces {
                        pickup: None,
                        delivery: Some(JobPlace {
                            times: Some(vec![
                                vec!["2020-07-04T09:00:00Z".to_owned(), "2020-07-04T13:00:00Z".to_owned()],
                                vec!["2020-07-04T14:00:00Z".to_owned(), "2020-07-04T18:00:00Z".to_owned()],
                            ]),
                            location: Location { lat: 77.0, lng: 0.0 },
                            duration: 15.0,
                            tag: None,
                        }),
                    },
                    demand: vec![2],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "74c9b5f9-2369-440b-bf0e-731aeba9ab8c".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T09:00:00Z".to_owned(),
                                "2020-07-04T11:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 50.0, lng: 0.0 },
                            duration: 13.0,
                            tag: None,
                        }),
                        delivery: None,
                    },
                    demand: vec![3],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "2a7273d4-ae56-4e3e-a2b1-a34b8e3c3263".to_owned(),
                    places: JobPlaces {
                        pickup: None,
                        delivery: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T09:00:00Z".to_owned(),
                                "2020-07-04T11:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 87.0, lng: 0.0 },
                            duration: 13.0,
                            tag: None,
                        }),
                    },
                    demand: vec![2],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "ead1f6df-780c-4bd2-88cf-d56b5b35dd54".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![
                                vec!["2020-07-04T09:00:00Z".to_owned(), "2020-07-04T13:00:00Z".to_owned()],
                                vec!["2020-07-04T14:00:00Z".to_owned(), "2020-07-04T16:00:00Z".to_owned()],
                            ]),
                            location: Location { lat: 88.0, lng: 0.0 },
                            duration: 15.0,
                            tag: None,
                        }),
                        delivery: None,
                    },
                    demand: vec![4],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "4e17bbc7-d0d8-4a42-8470-3b6eac942002".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![
                                vec!["2020-07-04T09:00:00Z".to_owned(), "2020-07-04T11:00:00Z".to_owned()],
                                vec!["2020-07-04T14:00:00Z".to_owned(), "2020-07-04T16:00:00Z".to_owned()],
                            ]),
                            location: Location { lat: 81.0, lng: 0.0 },
                            duration: 13.0,
                            tag: None,
                        }),
                        delivery: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T09:00:00Z".to_owned(),
                                "2020-07-04T13:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 18.0, lng: 0.0 },
                            duration: 18.0,
                            tag: None,
                        }),
                    },
                    demand: vec![3],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "98b2a0c6-8aa0-445e-b211-2b977dbc346a".to_owned(),
                    places: JobPlaces {
                        pickup: None,
                        delivery: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T16:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 94.0, lng: 0.0 },
                            duration: 11.0,
                            tag: None,
                        }),
                    },
                    demand: vec![3],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "2574d982-9337-41cb-a06c-5aeacc15d843".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T18:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 46.0, lng: 0.0 },
                            duration: 10.0,
                            tag: None,
                        }),
                        delivery: None,
                    },
                    demand: vec![3],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "f87340a7-4564-4b6c-8dcc-228ddda4dc45".to_owned(),
                    places: JobPlaces {
                        pickup: None,
                        delivery: Some(JobPlace {
                            times: Some(vec![
                                vec!["2020-07-04T09:00:00Z".to_owned(), "2020-07-04T11:00:00Z".to_owned()],
                                vec!["2020-07-04T14:00:00Z".to_owned(), "2020-07-04T16:00:00Z".to_owned()],
                            ]),
                            location: Location { lat: 77.0, lng: 0.0 },
                            duration: 15.0,
                            tag: None,
                        }),
                    },
                    demand: vec![2],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "5d8a3fcf-f1f5-4c9f-bfbe-e41b017031ef".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![
                                vec!["2020-07-04T09:00:00Z".to_owned(), "2020-07-04T13:00:00Z".to_owned()],
                                vec!["2020-07-04T14:00:00Z".to_owned(), "2020-07-04T18:00:00Z".to_owned()],
                            ]),
                            location: Location { lat: 48.0, lng: 0.0 },
                            duration: 10.0,
                            tag: None,
                        }),
                        delivery: None,
                    },
                    demand: vec![1],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "f0480e4f-b340-449a-bea3-5d714af36acc".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T16:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 34.0, lng: 0.0 },
                            duration: 18.0,
                            tag: None,
                        }),
                        delivery: None,
                    },
                    demand: vec![4],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "73b5ce8e-2b68-44d3-a394-a43a77da8030".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![
                                vec!["2020-07-04T09:00:00Z".to_owned(), "2020-07-04T11:00:00Z".to_owned()],
                                vec!["2020-07-04T14:00:00Z".to_owned(), "2020-07-04T18:00:00Z".to_owned()],
                            ]),
                            location: Location { lat: 12.0, lng: 0.0 },
                            duration: 16.0,
                            tag: None,
                        }),
                        delivery: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T09:00:00Z".to_owned(),
                                "2020-07-04T11:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 96.0, lng: 0.0 },
                            duration: 14.0,
                            tag: None,
                        }),
                    },
                    demand: vec![1],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "0af0e129-d807-41f0-b610-753528bb8c0c".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T09:00:00Z".to_owned(),
                                "2020-07-04T13:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 15.0, lng: 0.0 },
                            duration: 18.0,
                            tag: None,
                        }),
                        delivery: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T16:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 51.0, lng: 0.0 },
                            duration: 12.0,
                            tag: None,
                        }),
                    },
                    demand: vec![1],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "f3a6190e-1feb-4c66-a8bf-16e965890dc1".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![
                                vec!["2020-07-04T09:00:00Z".to_owned(), "2020-07-04T13:00:00Z".to_owned()],
                                vec!["2020-07-04T14:00:00Z".to_owned(), "2020-07-04T16:00:00Z".to_owned()],
                            ]),
                            location: Location { lat: 73.0, lng: 0.0 },
                            duration: 13.0,
                            tag: None,
                        }),
                        delivery: None,
                    },
                    demand: vec![1],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "0f0a9ab5-ab0d-468b-9575-9f35cad90a10".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T09:00:00Z".to_owned(),
                                "2020-07-04T13:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 55.0, lng: 0.0 },
                            duration: 15.0,
                            tag: None,
                        }),
                        delivery: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T18:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 92.0, lng: 0.0 },
                            duration: 14.0,
                            tag: None,
                        }),
                    },
                    demand: vec![1],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "12098647-8176-47ee-9f80-135063904f2f".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T09:00:00Z".to_owned(),
                                "2020-07-04T11:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 36.0, lng: 0.0 },
                            duration: 19.0,
                            tag: None,
                        }),
                        delivery: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T16:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 93.0, lng: 0.0 },
                            duration: 11.0,
                            tag: None,
                        }),
                    },
                    demand: vec![3],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "a053e149-cc85-40e9-acfb-06f4d360a6f6".to_owned(),
                    places: JobPlaces {
                        pickup: None,
                        delivery: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T16:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 57.0, lng: 0.0 },
                            duration: 19.0,
                            tag: None,
                        }),
                    },
                    demand: vec![4],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "ed2b7281-5d93-46d1-b3cc-efe1f8ed1a10".to_owned(),
                    places: JobPlaces {
                        pickup: None,
                        delivery: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T09:00:00Z".to_owned(),
                                "2020-07-04T11:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 12.0, lng: 0.0 },
                            duration: 16.0,
                            tag: None,
                        }),
                    },
                    demand: vec![2],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "47a861e6-e8d0-4f71-88de-ff77fab794f0".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T16:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 47.0, lng: 0.0 },
                            duration: 14.0,
                            tag: None,
                        }),
                        delivery: None,
                    },
                    demand: vec![4],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "b49f4246-5dfd-4097-97ff-e91db92da193".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T09:00:00Z".to_owned(),
                                "2020-07-04T13:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 52.0, lng: 0.0 },
                            duration: 18.0,
                            tag: None,
                        }),
                        delivery: Some(JobPlace {
                            times: Some(vec![
                                vec!["2020-07-04T09:00:00Z".to_owned(), "2020-07-04T13:00:00Z".to_owned()],
                                vec!["2020-07-04T14:00:00Z".to_owned(), "2020-07-04T18:00:00Z".to_owned()],
                            ]),
                            location: Location { lat: 20.0, lng: 0.0 },
                            duration: 16.0,
                            tag: None,
                        }),
                    },
                    demand: vec![1],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "29edd571-af3b-4c69-b675-88a510b4a81f".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T16:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 87.0, lng: 0.0 },
                            duration: 14.0,
                            tag: None,
                        }),
                        delivery: Some(JobPlace {
                            times: Some(vec![
                                vec!["2020-07-04T09:00:00Z".to_owned(), "2020-07-04T13:00:00Z".to_owned()],
                                vec!["2020-07-04T14:00:00Z".to_owned(), "2020-07-04T18:00:00Z".to_owned()],
                            ]),
                            location: Location { lat: 12.0, lng: 0.0 },
                            duration: 16.0,
                            tag: None,
                        }),
                    },
                    demand: vec![1],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "f8dec847-3a4b-406e-a507-99807b5051a1".to_owned(),
                    places: JobPlaces {
                        pickup: None,
                        delivery: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T09:00:00Z".to_owned(),
                                "2020-07-04T13:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 67.0, lng: 0.0 },
                            duration: 18.0,
                            tag: None,
                        }),
                    },
                    demand: vec![4],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "a4f67315-e0b1-453e-9c45-0e7ccad63068".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T18:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 77.0, lng: 0.0 },
                            duration: 10.0,
                            tag: None,
                        }),
                        delivery: None,
                    },
                    demand: vec![1],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "62e5f844-6a9b-4d31-875b-460d5cd0307b".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T18:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 9.0, lng: 0.0 },
                            duration: 14.0,
                            tag: None,
                        }),
                        delivery: None,
                    },
                    demand: vec![3],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "10823a94-4a3b-4c85-a370-6139729f6921".to_owned(),
                    places: JobPlaces {
                        pickup: None,
                        delivery: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T18:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 46.0, lng: 0.0 },
                            duration: 16.0,
                            tag: None,
                        }),
                    },
                    demand: vec![2],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "fbf321d5-29f5-4078-9e99-afcdfe5231cf".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![
                                vec!["2020-07-04T09:00:00Z".to_owned(), "2020-07-04T11:00:00Z".to_owned()],
                                vec!["2020-07-04T14:00:00Z".to_owned(), "2020-07-04T16:00:00Z".to_owned()],
                            ]),
                            location: Location { lat: 53.0, lng: 0.0 },
                            duration: 11.0,
                            tag: None,
                        }),
                        delivery: None,
                    },
                    demand: vec![3],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "ac9ca136-9fd8-4931-b316-b537a708542b".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T09:00:00Z".to_owned(),
                                "2020-07-04T13:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 52.0, lng: 0.0 },
                            duration: 15.0,
                            tag: None,
                        }),
                        delivery: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T16:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 67.0, lng: 0.0 },
                            duration: 17.0,
                            tag: None,
                        }),
                    },
                    demand: vec![2],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "88569689-1004-4f4d-ac1f-240528fba327".to_owned(),
                    places: JobPlaces {
                        pickup: None,
                        delivery: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T09:00:00Z".to_owned(),
                                "2020-07-04T13:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 69.0, lng: 0.0 },
                            duration: 14.0,
                            tag: None,
                        }),
                    },
                    demand: vec![1],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "b6aa8a0d-cc75-49b9-8a26-2ecebeb95f73".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T09:00:00Z".to_owned(),
                                "2020-07-04T11:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 66.0, lng: 0.0 },
                            duration: 18.0,
                            tag: None,
                        }),
                        delivery: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T09:00:00Z".to_owned(),
                                "2020-07-04T11:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 76.0, lng: 0.0 },
                            duration: 18.0,
                            tag: None,
                        }),
                    },
                    demand: vec![2],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "f5f41524-3fea-4dcb-91c0-16f34a123044".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![
                                vec!["2020-07-04T09:00:00Z".to_owned(), "2020-07-04T13:00:00Z".to_owned()],
                                vec!["2020-07-04T14:00:00Z".to_owned(), "2020-07-04T18:00:00Z".to_owned()],
                            ]),
                            location: Location { lat: 75.0, lng: 0.0 },
                            duration: 13.0,
                            tag: None,
                        }),
                        delivery: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T18:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 39.0, lng: 0.0 },
                            duration: 16.0,
                            tag: None,
                        }),
                    },
                    demand: vec![2],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "ff4bc163-b29f-4671-aed3-f71a178bd91c".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![
                                vec!["2020-07-04T09:00:00Z".to_owned(), "2020-07-04T11:00:00Z".to_owned()],
                                vec!["2020-07-04T14:00:00Z".to_owned(), "2020-07-04T18:00:00Z".to_owned()],
                            ]),
                            location: Location { lat: 70.0, lng: 0.0 },
                            duration: 10.0,
                            tag: None,
                        }),
                        delivery: None,
                    },
                    demand: vec![2],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "6f47aa68-0d8e-4aaa-b95e-94f44da17c30".to_owned(),
                    places: JobPlaces {
                        pickup: None,
                        delivery: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T09:00:00Z".to_owned(),
                                "2020-07-04T11:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 31.0, lng: 0.0 },
                            duration: 12.0,
                            tag: None,
                        }),
                    },
                    demand: vec![2],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "3138c544-df2c-43bf-8fda-ffaf3dcdc1ef".to_owned(),
                    places: JobPlaces {
                        pickup: None,
                        delivery: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T09:00:00Z".to_owned(),
                                "2020-07-04T11:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 58.0, lng: 0.0 },
                            duration: 11.0,
                            tag: None,
                        }),
                    },
                    demand: vec![2],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "6ac0ec4f-00fd-4bab-9d2a-a0e14afcb12b".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![
                                vec!["2020-07-04T09:00:00Z".to_owned(), "2020-07-04T13:00:00Z".to_owned()],
                                vec!["2020-07-04T14:00:00Z".to_owned(), "2020-07-04T18:00:00Z".to_owned()],
                            ]),
                            location: Location { lat: 46.0, lng: 0.0 },
                            duration: 19.0,
                            tag: None,
                        }),
                        delivery: None,
                    },
                    demand: vec![4],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "ff9db9ba-3d1b-4941-bc94-38db0faf86a6".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T18:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 58.0, lng: 0.0 },
                            duration: 12.0,
                            tag: None,
                        }),
                        delivery: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T18:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 58.0, lng: 0.0 },
                            duration: 12.0,
                            tag: None,
                        }),
                    },
                    demand: vec![2],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "99cfd541-b188-4f68-aabb-3f0986018e87".to_owned(),
                    places: JobPlaces {
                        pickup: None,
                        delivery: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T09:00:00Z".to_owned(),
                                "2020-07-04T11:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 35.0, lng: 0.0 },
                            duration: 15.0,
                            tag: None,
                        }),
                    },
                    demand: vec![3],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "9e461662-7ac5-42af-aab4-20c4b1865bb6".to_owned(),
                    places: JobPlaces {
                        pickup: None,
                        delivery: Some(JobPlace {
                            times: Some(vec![
                                vec!["2020-07-04T09:00:00Z".to_owned(), "2020-07-04T13:00:00Z".to_owned()],
                                vec!["2020-07-04T14:00:00Z".to_owned(), "2020-07-04T16:00:00Z".to_owned()],
                            ]),
                            location: Location { lat: 40.0, lng: 0.0 },
                            duration: 11.0,
                            tag: None,
                        }),
                    },
                    demand: vec![3],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "8494a520-fa64-4220-9cfd-c873654758a8".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T18:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 87.0, lng: 0.0 },
                            duration: 16.0,
                            tag: None,
                        }),
                        delivery: None,
                    },
                    demand: vec![1],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "6978fec3-c14a-4191-88ca-0e5b2f62aec7".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![
                                vec!["2020-07-04T09:00:00Z".to_owned(), "2020-07-04T13:00:00Z".to_owned()],
                                vec!["2020-07-04T14:00:00Z".to_owned(), "2020-07-04T18:00:00Z".to_owned()],
                            ]),
                            location: Location { lat: 62.0, lng: 0.0 },
                            duration: 11.0,
                            tag: None,
                        }),
                        delivery: None,
                    },
                    demand: vec![2],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "f109a922-5b9a-401c-b988-ea8d232df3f2".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![
                                vec!["2020-07-04T09:00:00Z".to_owned(), "2020-07-04T11:00:00Z".to_owned()],
                                vec!["2020-07-04T14:00:00Z".to_owned(), "2020-07-04T18:00:00Z".to_owned()],
                            ]),
                            location: Location { lat: 25.0, lng: 0.0 },
                            duration: 13.0,
                            tag: None,
                        }),
                        delivery: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T16:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 49.0, lng: 0.0 },
                            duration: 12.0,
                            tag: None,
                        }),
                    },
                    demand: vec![1],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "b60f3576-a645-413e-bca2-9e60e7ec7b49".to_owned(),
                    places: JobPlaces {
                        pickup: None,
                        delivery: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T09:00:00Z".to_owned(),
                                "2020-07-04T13:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 32.0, lng: 0.0 },
                            duration: 14.0,
                            tag: None,
                        }),
                    },
                    demand: vec![2],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "09a60f71-0e00-4540-84a2-5e7060afa73a".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T16:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 7.0, lng: 0.0 },
                            duration: 13.0,
                            tag: None,
                        }),
                        delivery: None,
                    },
                    demand: vec![3],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "b27b25ae-97f6-4a52-b52d-01eacf8cd306".to_owned(),
                    places: JobPlaces {
                        pickup: None,
                        delivery: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T09:00:00Z".to_owned(),
                                "2020-07-04T11:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 26.0, lng: 0.0 },
                            duration: 14.0,
                            tag: None,
                        }),
                    },
                    demand: vec![2],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "aef6666c-90ef-48b3-a916-8326fc201baa".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T18:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 23.0, lng: 0.0 },
                            duration: 17.0,
                            tag: None,
                        }),
                        delivery: None,
                    },
                    demand: vec![1],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "f3436c26-e4a9-482c-b3aa-d28e07d4afcb".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T16:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 27.0, lng: 0.0 },
                            duration: 19.0,
                            tag: None,
                        }),
                        delivery: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T16:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 37.0, lng: 0.0 },
                            duration: 11.0,
                            tag: None,
                        }),
                    },
                    demand: vec![3],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "2e3dd3d3-6a9d-4c03-8194-61329240b896".to_owned(),
                    places: JobPlaces {
                        pickup: None,
                        delivery: Some(JobPlace {
                            times: Some(vec![
                                vec!["2020-07-04T09:00:00Z".to_owned(), "2020-07-04T13:00:00Z".to_owned()],
                                vec!["2020-07-04T14:00:00Z".to_owned(), "2020-07-04T16:00:00Z".to_owned()],
                            ]),
                            location: Location { lat: 52.0, lng: 0.0 },
                            duration: 10.0,
                            tag: None,
                        }),
                    },
                    demand: vec![3],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "8ab32f3f-a6b9-4f68-9260-e8b930cb5f7d".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T16:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 68.0, lng: 0.0 },
                            duration: 10.0,
                            tag: None,
                        }),
                        delivery: None,
                    },
                    demand: vec![4],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "c78eb5aa-be09-418c-851f-29e120a0dea2".to_owned(),
                    places: JobPlaces {
                        pickup: None,
                        delivery: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T16:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 37.0, lng: 0.0 },
                            duration: 15.0,
                            tag: None,
                        }),
                    },
                    demand: vec![3],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "94c5b540-0b5b-4db9-933c-34c041e24138".to_owned(),
                    places: JobPlaces {
                        pickup: None,
                        delivery: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T18:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 3.0, lng: 0.0 },
                            duration: 15.0,
                            tag: None,
                        }),
                    },
                    demand: vec![1],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "bf3098a5-57bc-4b6b-a5bf-0d88feb7de9d".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T09:00:00Z".to_owned(),
                                "2020-07-04T13:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 86.0, lng: 0.0 },
                            duration: 12.0,
                            tag: None,
                        }),
                        delivery: None,
                    },
                    demand: vec![4],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "36a7a3f4-fb92-4fc7-add1-1c9a27cef39e".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T09:00:00Z".to_owned(),
                                "2020-07-04T11:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 19.0, lng: 0.0 },
                            duration: 17.0,
                            tag: None,
                        }),
                        delivery: None,
                    },
                    demand: vec![2],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "7136687f-b428-48e0-aa24-fec50c062dad".to_owned(),
                    places: JobPlaces {
                        pickup: None,
                        delivery: Some(JobPlace {
                            times: Some(vec![
                                vec!["2020-07-04T09:00:00Z".to_owned(), "2020-07-04T13:00:00Z".to_owned()],
                                vec!["2020-07-04T14:00:00Z".to_owned(), "2020-07-04T18:00:00Z".to_owned()],
                            ]),
                            location: Location { lat: 2.0, lng: 0.0 },
                            duration: 13.0,
                            tag: None,
                        }),
                    },
                    demand: vec![3],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "f1b8007b-129c-4267-867b-1ed1c6f3647d".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T09:00:00Z".to_owned(),
                                "2020-07-04T11:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 57.0, lng: 0.0 },
                            duration: 12.0,
                            tag: None,
                        }),
                        delivery: None,
                    },
                    demand: vec![3],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "b371993c-0ec4-43ce-b71d-d1f90bdb552f".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T09:00:00Z".to_owned(),
                                "2020-07-04T13:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 96.0, lng: 0.0 },
                            duration: 19.0,
                            tag: None,
                        }),
                        delivery: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T09:00:00Z".to_owned(),
                                "2020-07-04T13:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 19.0, lng: 0.0 },
                            duration: 11.0,
                            tag: None,
                        }),
                    },
                    demand: vec![3],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "9d77c075-ef8e-48d5-ad9a-6ddae9278fd0".to_owned(),
                    places: JobPlaces {
                        pickup: None,
                        delivery: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T09:00:00Z".to_owned(),
                                "2020-07-04T11:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 63.0, lng: 0.0 },
                            duration: 16.0,
                            tag: None,
                        }),
                    },
                    demand: vec![3],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "e683c365-04a8-4f06-8547-f1fb4c6f27f7".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T09:00:00Z".to_owned(),
                                "2020-07-04T13:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 24.0, lng: 0.0 },
                            duration: 13.0,
                            tag: None,
                        }),
                        delivery: None,
                    },
                    demand: vec![3],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "8df4868f-2e8d-4e5e-b68d-485d270ec5a5".to_owned(),
                    places: JobPlaces {
                        pickup: None,
                        delivery: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T18:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 73.0, lng: 0.0 },
                            duration: 11.0,
                            tag: None,
                        }),
                    },
                    demand: vec![3],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "2edc2f36-207c-4b70-9afd-850a2445e8b8".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T09:00:00Z".to_owned(),
                                "2020-07-04T11:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 75.0, lng: 0.0 },
                            duration: 14.0,
                            tag: None,
                        }),
                        delivery: None,
                    },
                    demand: vec![4],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "f2d37fbd-11ee-4ce0-8c75-455e46d4fcc6".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T16:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 88.0, lng: 0.0 },
                            duration: 14.0,
                            tag: None,
                        }),
                        delivery: None,
                    },
                    demand: vec![3],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "4a9c1cf0-6e63-494f-a8ec-e47485395f4f".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T18:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 45.0, lng: 0.0 },
                            duration: 15.0,
                            tag: None,
                        }),
                        delivery: Some(JobPlace {
                            times: Some(vec![
                                vec!["2020-07-04T09:00:00Z".to_owned(), "2020-07-04T11:00:00Z".to_owned()],
                                vec!["2020-07-04T14:00:00Z".to_owned(), "2020-07-04T16:00:00Z".to_owned()],
                            ]),
                            location: Location { lat: 57.0, lng: 0.0 },
                            duration: 11.0,
                            tag: None,
                        }),
                    },
                    demand: vec![3],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "09c0f8ac-886e-46bd-97c2-f3d1e0978748".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T16:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 12.0, lng: 0.0 },
                            duration: 14.0,
                            tag: None,
                        }),
                        delivery: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T09:00:00Z".to_owned(),
                                "2020-07-04T11:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 10.0, lng: 0.0 },
                            duration: 10.0,
                            tag: None,
                        }),
                    },
                    demand: vec![1],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "0d145b51-7e66-4398-b656-3753fb467006".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![
                                vec!["2020-07-04T09:00:00Z".to_owned(), "2020-07-04T11:00:00Z".to_owned()],
                                vec!["2020-07-04T14:00:00Z".to_owned(), "2020-07-04T18:00:00Z".to_owned()],
                            ]),
                            location: Location { lat: 8.0, lng: 0.0 },
                            duration: 18.0,
                            tag: None,
                        }),
                        delivery: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T18:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 52.0, lng: 0.0 },
                            duration: 14.0,
                            tag: None,
                        }),
                    },
                    demand: vec![2],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "11e7486d-2e2e-4073-8ad8-c50e78d4536a".to_owned(),
                    places: JobPlaces {
                        pickup: None,
                        delivery: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T18:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 3.0, lng: 0.0 },
                            duration: 11.0,
                            tag: None,
                        }),
                    },
                    demand: vec![3],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "4d27e536-de62-492b-92bc-271c6872ecbb".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![
                                vec!["2020-07-04T09:00:00Z".to_owned(), "2020-07-04T13:00:00Z".to_owned()],
                                vec!["2020-07-04T14:00:00Z".to_owned(), "2020-07-04T16:00:00Z".to_owned()],
                            ]),
                            location: Location { lat: 78.0, lng: 0.0 },
                            duration: 18.0,
                            tag: None,
                        }),
                        delivery: None,
                    },
                    demand: vec![3],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "2ad4135b-c524-4f3a-a271-6a0817349575".to_owned(),
                    places: JobPlaces {
                        pickup: None,
                        delivery: Some(JobPlace {
                            times: Some(vec![
                                vec!["2020-07-04T09:00:00Z".to_owned(), "2020-07-04T11:00:00Z".to_owned()],
                                vec!["2020-07-04T14:00:00Z".to_owned(), "2020-07-04T16:00:00Z".to_owned()],
                            ]),
                            location: Location { lat: 96.0, lng: 0.0 },
                            duration: 13.0,
                            tag: None,
                        }),
                    },
                    demand: vec![4],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "1f6999e3-3828-4f26-8c96-8f9844030ec6".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![
                                vec!["2020-07-04T09:00:00Z".to_owned(), "2020-07-04T13:00:00Z".to_owned()],
                                vec!["2020-07-04T14:00:00Z".to_owned(), "2020-07-04T18:00:00Z".to_owned()],
                            ]),
                            location: Location { lat: 45.0, lng: 0.0 },
                            duration: 18.0,
                            tag: None,
                        }),
                        delivery: None,
                    },
                    demand: vec![3],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "7ec6e67f-3659-4971-90af-88e4c872dc42".to_owned(),
                    places: JobPlaces {
                        pickup: None,
                        delivery: Some(JobPlace {
                            times: Some(vec![
                                vec!["2020-07-04T09:00:00Z".to_owned(), "2020-07-04T13:00:00Z".to_owned()],
                                vec!["2020-07-04T14:00:00Z".to_owned(), "2020-07-04T18:00:00Z".to_owned()],
                            ]),
                            location: Location { lat: 17.0, lng: 0.0 },
                            duration: 17.0,
                            tag: None,
                        }),
                    },
                    demand: vec![3],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "250adc91-b62f-4893-8829-55a2dbaf7f2d".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T18:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 96.0, lng: 0.0 },
                            duration: 14.0,
                            tag: None,
                        }),
                        delivery: None,
                    },
                    demand: vec![2],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "b26b9d3d-aea9-4fed-89d2-376edcf39fbd".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T09:00:00Z".to_owned(),
                                "2020-07-04T13:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 43.0, lng: 0.0 },
                            duration: 15.0,
                            tag: None,
                        }),
                        delivery: None,
                    },
                    demand: vec![3],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "2476a215-1e67-401a-b802-f586fb03eff9".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T16:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 66.0, lng: 0.0 },
                            duration: 12.0,
                            tag: None,
                        }),
                        delivery: None,
                    },
                    demand: vec![2],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "0eef37f6-5ec8-45b2-9ce9-d67227ee8be6".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T09:00:00Z".to_owned(),
                                "2020-07-04T13:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 46.0, lng: 0.0 },
                            duration: 12.0,
                            tag: None,
                        }),
                        delivery: None,
                    },
                    demand: vec![3],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "bd033952-c5d4-4487-b400-0e9542913b17".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![
                                vec!["2020-07-04T09:00:00Z".to_owned(), "2020-07-04T11:00:00Z".to_owned()],
                                vec!["2020-07-04T14:00:00Z".to_owned(), "2020-07-04T18:00:00Z".to_owned()],
                            ]),
                            location: Location { lat: 22.0, lng: 0.0 },
                            duration: 15.0,
                            tag: None,
                        }),
                        delivery: Some(JobPlace {
                            times: Some(vec![
                                vec!["2020-07-04T09:00:00Z".to_owned(), "2020-07-04T11:00:00Z".to_owned()],
                                vec!["2020-07-04T14:00:00Z".to_owned(), "2020-07-04T18:00:00Z".to_owned()],
                            ]),
                            location: Location { lat: 33.0, lng: 0.0 },
                            duration: 15.0,
                            tag: None,
                        }),
                    },
                    demand: vec![1],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "d4064ece-30ef-44ef-815f-25adb383a801".to_owned(),
                    places: JobPlaces {
                        pickup: None,
                        delivery: Some(JobPlace {
                            times: Some(vec![
                                vec!["2020-07-04T09:00:00Z".to_owned(), "2020-07-04T11:00:00Z".to_owned()],
                                vec!["2020-07-04T14:00:00Z".to_owned(), "2020-07-04T18:00:00Z".to_owned()],
                            ]),
                            location: Location { lat: 81.0, lng: 0.0 },
                            duration: 19.0,
                            tag: None,
                        }),
                    },
                    demand: vec![1],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "0eeac139-d8b7-4f5a-9226-c0bf78cf470c".to_owned(),
                    places: JobPlaces {
                        pickup: None,
                        delivery: Some(JobPlace {
                            times: Some(vec![
                                vec!["2020-07-04T09:00:00Z".to_owned(), "2020-07-04T13:00:00Z".to_owned()],
                                vec!["2020-07-04T14:00:00Z".to_owned(), "2020-07-04T18:00:00Z".to_owned()],
                            ]),
                            location: Location { lat: 35.0, lng: 0.0 },
                            duration: 16.0,
                            tag: None,
                        }),
                    },
                    demand: vec![1],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "1e531b61-d26e-4b39-994e-dc9a30291807".to_owned(),
                    places: JobPlaces {
                        pickup: None,
                        delivery: Some(JobPlace {
                            times: Some(vec![
                                vec!["2020-07-04T09:00:00Z".to_owned(), "2020-07-04T11:00:00Z".to_owned()],
                                vec!["2020-07-04T14:00:00Z".to_owned(), "2020-07-04T18:00:00Z".to_owned()],
                            ]),
                            location: Location { lat: 58.0, lng: 0.0 },
                            duration: 14.0,
                            tag: None,
                        }),
                    },
                    demand: vec![2],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "eb945053-648a-4f1b-8231-381afd2f4185".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![
                                vec!["2020-07-04T09:00:00Z".to_owned(), "2020-07-04T11:00:00Z".to_owned()],
                                vec!["2020-07-04T14:00:00Z".to_owned(), "2020-07-04T16:00:00Z".to_owned()],
                            ]),
                            location: Location { lat: 6.0, lng: 0.0 },
                            duration: 16.0,
                            tag: None,
                        }),
                        delivery: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T09:00:00Z".to_owned(),
                                "2020-07-04T11:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 79.0, lng: 0.0 },
                            duration: 15.0,
                            tag: None,
                        }),
                    },
                    demand: vec![1],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "ced60c78-09f1-4094-b0f9-eeadff1f9294".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![
                                vec!["2020-07-04T09:00:00Z".to_owned(), "2020-07-04T11:00:00Z".to_owned()],
                                vec!["2020-07-04T14:00:00Z".to_owned(), "2020-07-04T16:00:00Z".to_owned()],
                            ]),
                            location: Location { lat: 4.0, lng: 0.0 },
                            duration: 13.0,
                            tag: None,
                        }),
                        delivery: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T09:00:00Z".to_owned(),
                                "2020-07-04T13:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 64.0, lng: 0.0 },
                            duration: 12.0,
                            tag: None,
                        }),
                    },
                    demand: vec![2],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "f18d3d50-680f-401d-9e32-6d52ff820014".to_owned(),
                    places: JobPlaces {
                        pickup: None,
                        delivery: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T16:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 41.0, lng: 0.0 },
                            duration: 14.0,
                            tag: None,
                        }),
                    },
                    demand: vec![2],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "5c0541c4-4278-4da7-ab90-d5bb60a3111b".to_owned(),
                    places: JobPlaces {
                        pickup: None,
                        delivery: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T09:00:00Z".to_owned(),
                                "2020-07-04T11:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 14.0, lng: 0.0 },
                            duration: 11.0,
                            tag: None,
                        }),
                    },
                    demand: vec![4],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "57b13a9f-8614-46ad-99f7-5b992ca9bb26".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![
                                vec!["2020-07-04T09:00:00Z".to_owned(), "2020-07-04T11:00:00Z".to_owned()],
                                vec!["2020-07-04T14:00:00Z".to_owned(), "2020-07-04T18:00:00Z".to_owned()],
                            ]),
                            location: Location { lat: 48.0, lng: 0.0 },
                            duration: 16.0,
                            tag: None,
                        }),
                        delivery: None,
                    },
                    demand: vec![1],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "dd533c1b-c0a6-4747-b660-bd6b204948fd".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T18:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 91.0, lng: 0.0 },
                            duration: 18.0,
                            tag: None,
                        }),
                        delivery: Some(JobPlace {
                            times: Some(vec![
                                vec!["2020-07-04T09:00:00Z".to_owned(), "2020-07-04T13:00:00Z".to_owned()],
                                vec!["2020-07-04T14:00:00Z".to_owned(), "2020-07-04T16:00:00Z".to_owned()],
                            ]),
                            location: Location { lat: 18.0, lng: 0.0 },
                            duration: 17.0,
                            tag: None,
                        }),
                    },
                    demand: vec![3],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "2f7ca4b4-aecf-458c-9a58-ac4ec95e5051".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T09:00:00Z".to_owned(),
                                "2020-07-04T11:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 96.0, lng: 0.0 },
                            duration: 11.0,
                            tag: None,
                        }),
                        delivery: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T09:00:00Z".to_owned(),
                                "2020-07-04T13:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 30.0, lng: 0.0 },
                            duration: 14.0,
                            tag: None,
                        }),
                    },
                    demand: vec![3],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "0b313dee-ddbe-4859-aeea-a12fe76440fa".to_owned(),
                    places: JobPlaces {
                        pickup: None,
                        delivery: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T18:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 71.0, lng: 0.0 },
                            duration: 18.0,
                            tag: None,
                        }),
                    },
                    demand: vec![3],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "03ea68e5-4b08-4813-8031-e90baf23621b".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T09:00:00Z".to_owned(),
                                "2020-07-04T13:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 23.0, lng: 0.0 },
                            duration: 18.0,
                            tag: None,
                        }),
                        delivery: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T18:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 12.0, lng: 0.0 },
                            duration: 16.0,
                            tag: None,
                        }),
                    },
                    demand: vec![1],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "18eb11da-c53a-480f-8464-5b721acdeda2".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T18:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 90.0, lng: 0.0 },
                            duration: 15.0,
                            tag: None,
                        }),
                        delivery: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T09:00:00Z".to_owned(),
                                "2020-07-04T13:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 80.0, lng: 0.0 },
                            duration: 18.0,
                            tag: None,
                        }),
                    },
                    demand: vec![1],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "a8912cc5-81e9-4d43-8158-95a5aef7cbbc".to_owned(),
                    places: JobPlaces {
                        pickup: None,
                        delivery: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T09:00:00Z".to_owned(),
                                "2020-07-04T13:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 17.0, lng: 0.0 },
                            duration: 14.0,
                            tag: None,
                        }),
                    },
                    demand: vec![4],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "b02e29f1-500b-4abd-91f9-46b73f34fc44".to_owned(),
                    places: JobPlaces {
                        pickup: None,
                        delivery: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T09:00:00Z".to_owned(),
                                "2020-07-04T13:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 8.0, lng: 0.0 },
                            duration: 11.0,
                            tag: None,
                        }),
                    },
                    demand: vec![4],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "3bd6ed87-8777-4553-80f1-03623c223be6".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![
                                vec!["2020-07-04T09:00:00Z".to_owned(), "2020-07-04T11:00:00Z".to_owned()],
                                vec!["2020-07-04T14:00:00Z".to_owned(), "2020-07-04T16:00:00Z".to_owned()],
                            ]),
                            location: Location { lat: 35.0, lng: 0.0 },
                            duration: 12.0,
                            tag: None,
                        }),
                        delivery: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T09:00:00Z".to_owned(),
                                "2020-07-04T11:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 59.0, lng: 0.0 },
                            duration: 15.0,
                            tag: None,
                        }),
                    },
                    demand: vec![3],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "9798f600-ef3a-4847-bc23-09006f3ab06f".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T18:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 10.0, lng: 0.0 },
                            duration: 14.0,
                            tag: None,
                        }),
                        delivery: None,
                    },
                    demand: vec![3],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "127d3857-9a84-4334-83d3-bf383cb4fbfe".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![
                                vec!["2020-07-04T09:00:00Z".to_owned(), "2020-07-04T13:00:00Z".to_owned()],
                                vec!["2020-07-04T14:00:00Z".to_owned(), "2020-07-04T18:00:00Z".to_owned()],
                            ]),
                            location: Location { lat: 30.0, lng: 0.0 },
                            duration: 14.0,
                            tag: None,
                        }),
                        delivery: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T18:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 89.0, lng: 0.0 },
                            duration: 18.0,
                            tag: None,
                        }),
                    },
                    demand: vec![3],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "672b5216-184c-450a-a633-c87774bbb033".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![
                                vec!["2020-07-04T09:00:00Z".to_owned(), "2020-07-04T13:00:00Z".to_owned()],
                                vec!["2020-07-04T14:00:00Z".to_owned(), "2020-07-04T16:00:00Z".to_owned()],
                            ]),
                            location: Location { lat: 35.0, lng: 0.0 },
                            duration: 17.0,
                            tag: None,
                        }),
                        delivery: None,
                    },
                    demand: vec![4],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "728549b9-1fa8-4acf-9749-244a3ac41948".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T09:00:00Z".to_owned(),
                                "2020-07-04T13:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 84.0, lng: 0.0 },
                            duration: 14.0,
                            tag: None,
                        }),
                        delivery: None,
                    },
                    demand: vec![2],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "223fa3dc-8c42-4e68-a9ab-d863e49812a4".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T09:00:00Z".to_owned(),
                                "2020-07-04T13:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 41.0, lng: 0.0 },
                            duration: 16.0,
                            tag: None,
                        }),
                        delivery: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T09:00:00Z".to_owned(),
                                "2020-07-04T11:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 67.0, lng: 0.0 },
                            duration: 16.0,
                            tag: None,
                        }),
                    },
                    demand: vec![1],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "c09124a7-58c1-4572-8f61-0f5c326bb010".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T09:00:00Z".to_owned(),
                                "2020-07-04T13:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 60.0, lng: 0.0 },
                            duration: 10.0,
                            tag: None,
                        }),
                        delivery: None,
                    },
                    demand: vec![4],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "8c13acfb-cfb0-4966-8497-b6f62d4544c4".to_owned(),
                    places: JobPlaces {
                        pickup: None,
                        delivery: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T09:00:00Z".to_owned(),
                                "2020-07-04T11:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 40.0, lng: 0.0 },
                            duration: 10.0,
                            tag: None,
                        }),
                    },
                    demand: vec![1],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "fe5c5467-be29-4eae-9d89-309277a94ebc".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T09:00:00Z".to_owned(),
                                "2020-07-04T13:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 11.0, lng: 0.0 },
                            duration: 12.0,
                            tag: None,
                        }),
                        delivery: None,
                    },
                    demand: vec![1],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "0c7dc671-7423-4d0f-b03e-7488e081a894".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T09:00:00Z".to_owned(),
                                "2020-07-04T13:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 70.0, lng: 0.0 },
                            duration: 16.0,
                            tag: None,
                        }),
                        delivery: None,
                    },
                    demand: vec![4],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "144bc4d7-8a37-40b3-99dd-a93d43156d72".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T16:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 5.0, lng: 0.0 },
                            duration: 14.0,
                            tag: None,
                        }),
                        delivery: None,
                    },
                    demand: vec![4],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "7deaa727-207c-4ad3-a27a-936b9e06f578".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T18:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 89.0, lng: 0.0 },
                            duration: 15.0,
                            tag: None,
                        }),
                        delivery: None,
                    },
                    demand: vec![1],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "7f554af0-6d3c-4ff2-b64b-7b4c49dc8f6d".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T09:00:00Z".to_owned(),
                                "2020-07-04T11:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 54.0, lng: 0.0 },
                            duration: 12.0,
                            tag: None,
                        }),
                        delivery: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T18:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 10.0, lng: 0.0 },
                            duration: 12.0,
                            tag: None,
                        }),
                    },
                    demand: vec![2],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "839b8e7a-f643-456f-8c2e-983a0a7535ec".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![
                                vec!["2020-07-04T09:00:00Z".to_owned(), "2020-07-04T11:00:00Z".to_owned()],
                                vec!["2020-07-04T14:00:00Z".to_owned(), "2020-07-04T18:00:00Z".to_owned()],
                            ]),
                            location: Location { lat: 62.0, lng: 0.0 },
                            duration: 14.0,
                            tag: None,
                        }),
                        delivery: None,
                    },
                    demand: vec![1],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "31c8c5ec-d85c-44a5-9877-5791adf7d1a9".to_owned(),
                    places: JobPlaces {
                        pickup: None,
                        delivery: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T16:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 31.0, lng: 0.0 },
                            duration: 12.0,
                            tag: None,
                        }),
                    },
                    demand: vec![4],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "0fc57959-c2b0-478a-ad86-4ecb9ec7b589".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T09:00:00Z".to_owned(),
                                "2020-07-04T11:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 51.0, lng: 0.0 },
                            duration: 19.0,
                            tag: None,
                        }),
                        delivery: None,
                    },
                    demand: vec![1],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "5292a795-6be1-47ed-879a-7c7f9980b115".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![
                                vec!["2020-07-04T09:00:00Z".to_owned(), "2020-07-04T11:00:00Z".to_owned()],
                                vec!["2020-07-04T14:00:00Z".to_owned(), "2020-07-04T16:00:00Z".to_owned()],
                            ]),
                            location: Location { lat: 38.0, lng: 0.0 },
                            duration: 12.0,
                            tag: None,
                        }),
                        delivery: None,
                    },
                    demand: vec![1],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "a387d1bc-7d2b-4a4b-b639-a8fab7e1b4c5".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T18:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 59.0, lng: 0.0 },
                            duration: 18.0,
                            tag: None,
                        }),
                        delivery: None,
                    },
                    demand: vec![1],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "c18abc7d-3031-44cd-8c34-11d503779e00".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![
                                vec!["2020-07-04T09:00:00Z".to_owned(), "2020-07-04T13:00:00Z".to_owned()],
                                vec!["2020-07-04T14:00:00Z".to_owned(), "2020-07-04T16:00:00Z".to_owned()],
                            ]),
                            location: Location { lat: 56.0, lng: 0.0 },
                            duration: 12.0,
                            tag: None,
                        }),
                        delivery: None,
                    },
                    demand: vec![4],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "41d8c0f1-2538-4a15-90c3-aa3cb54a6b38".to_owned(),
                    places: JobPlaces {
                        pickup: None,
                        delivery: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T16:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 24.0, lng: 0.0 },
                            duration: 13.0,
                            tag: None,
                        }),
                    },
                    demand: vec![3],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "f6270722-cc36-4604-91a4-87ceea26100d".to_owned(),
                    places: JobPlaces {
                        pickup: None,
                        delivery: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T09:00:00Z".to_owned(),
                                "2020-07-04T11:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 73.0, lng: 0.0 },
                            duration: 17.0,
                            tag: None,
                        }),
                    },
                    demand: vec![4],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "b274151c-1bed-432c-bb66-c04806342184".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T18:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 31.0, lng: 0.0 },
                            duration: 10.0,
                            tag: None,
                        }),
                        delivery: None,
                    },
                    demand: vec![2],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "eb2a3c35-01d0-4462-b371-9c8608058744".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T16:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 97.0, lng: 0.0 },
                            duration: 13.0,
                            tag: None,
                        }),
                        delivery: Some(JobPlace {
                            times: Some(vec![
                                vec!["2020-07-04T09:00:00Z".to_owned(), "2020-07-04T11:00:00Z".to_owned()],
                                vec!["2020-07-04T14:00:00Z".to_owned(), "2020-07-04T18:00:00Z".to_owned()],
                            ]),
                            location: Location { lat: 98.0, lng: 0.0 },
                            duration: 19.0,
                            tag: None,
                        }),
                    },
                    demand: vec![2],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "e7eda27b-b508-4e18-b2c7-c377c01ef734".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T18:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 90.0, lng: 0.0 },
                            duration: 19.0,
                            tag: None,
                        }),
                        delivery: None,
                    },
                    demand: vec![1],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "4475670b-b6d3-49e5-bd54-63233b828791".to_owned(),
                    places: JobPlaces {
                        pickup: None,
                        delivery: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T18:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 81.0, lng: 0.0 },
                            duration: 19.0,
                            tag: None,
                        }),
                    },
                    demand: vec![3],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "67ddde7e-a94b-450b-945b-e88c8deb36d4".to_owned(),
                    places: JobPlaces {
                        pickup: None,
                        delivery: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T18:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 68.0, lng: 0.0 },
                            duration: 18.0,
                            tag: None,
                        }),
                    },
                    demand: vec![3],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "3d01506d-2038-4677-9f85-50105d51526e".to_owned(),
                    places: JobPlaces {
                        pickup: None,
                        delivery: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T18:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 84.0, lng: 0.0 },
                            duration: 10.0,
                            tag: None,
                        }),
                    },
                    demand: vec![3],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "fe3fcd1f-f109-4c84-a686-192b6ea505ba".to_owned(),
                    places: JobPlaces {
                        pickup: None,
                        delivery: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T18:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 35.0, lng: 0.0 },
                            duration: 18.0,
                            tag: None,
                        }),
                    },
                    demand: vec![4],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "ec5b17d4-49fd-4c22-b744-9167ab2b0e0a".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T18:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 26.0, lng: 0.0 },
                            duration: 16.0,
                            tag: None,
                        }),
                        delivery: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T18:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 61.0, lng: 0.0 },
                            duration: 15.0,
                            tag: None,
                        }),
                    },
                    demand: vec![1],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "235a47d7-2bca-4e98-a7f1-469046f71656".to_owned(),
                    places: JobPlaces {
                        pickup: None,
                        delivery: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T16:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 30.0, lng: 0.0 },
                            duration: 17.0,
                            tag: None,
                        }),
                    },
                    demand: vec![4],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "f98cb4b8-003d-48f6-87d8-3b288cb822b0".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![
                                vec!["2020-07-04T09:00:00Z".to_owned(), "2020-07-04T11:00:00Z".to_owned()],
                                vec!["2020-07-04T14:00:00Z".to_owned(), "2020-07-04T18:00:00Z".to_owned()],
                            ]),
                            location: Location { lat: 2.0, lng: 0.0 },
                            duration: 17.0,
                            tag: None,
                        }),
                        delivery: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T18:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 26.0, lng: 0.0 },
                            duration: 11.0,
                            tag: None,
                        }),
                    },
                    demand: vec![3],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "1f90824c-9406-4ec8-8341-93472e0aaefe".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T09:00:00Z".to_owned(),
                                "2020-07-04T13:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 27.0, lng: 0.0 },
                            duration: 13.0,
                            tag: None,
                        }),
                        delivery: Some(JobPlace {
                            times: Some(vec![
                                vec!["2020-07-04T09:00:00Z".to_owned(), "2020-07-04T11:00:00Z".to_owned()],
                                vec!["2020-07-04T14:00:00Z".to_owned(), "2020-07-04T16:00:00Z".to_owned()],
                            ]),
                            location: Location { lat: 15.0, lng: 0.0 },
                            duration: 19.0,
                            tag: None,
                        }),
                    },
                    demand: vec![1],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "2f24e19b-47fb-42b0-9525-f5fc5cc9ca05".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T09:00:00Z".to_owned(),
                                "2020-07-04T13:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 75.0, lng: 0.0 },
                            duration: 15.0,
                            tag: None,
                        }),
                        delivery: Some(JobPlace {
                            times: Some(vec![
                                vec!["2020-07-04T09:00:00Z".to_owned(), "2020-07-04T11:00:00Z".to_owned()],
                                vec!["2020-07-04T14:00:00Z".to_owned(), "2020-07-04T18:00:00Z".to_owned()],
                            ]),
                            location: Location { lat: 15.0, lng: 0.0 },
                            duration: 11.0,
                            tag: None,
                        }),
                    },
                    demand: vec![2],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "3782a6b3-f93a-45e0-aef7-12fad9426591".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T18:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 5.0, lng: 0.0 },
                            duration: 19.0,
                            tag: None,
                        }),
                        delivery: None,
                    },
                    demand: vec![3],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "6c6b5437-32f3-4dc3-922d-0ced2b8c4aa0".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T16:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 66.0, lng: 0.0 },
                            duration: 16.0,
                            tag: None,
                        }),
                        delivery: Some(JobPlace {
                            times: Some(vec![
                                vec!["2020-07-04T09:00:00Z".to_owned(), "2020-07-04T13:00:00Z".to_owned()],
                                vec!["2020-07-04T14:00:00Z".to_owned(), "2020-07-04T16:00:00Z".to_owned()],
                            ]),
                            location: Location { lat: 64.0, lng: 0.0 },
                            duration: 15.0,
                            tag: None,
                        }),
                    },
                    demand: vec![1],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "9578ff72-8e07-4b62-8b4c-6e054b9af657".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T09:00:00Z".to_owned(),
                                "2020-07-04T11:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 11.0, lng: 0.0 },
                            duration: 18.0,
                            tag: None,
                        }),
                        delivery: None,
                    },
                    demand: vec![4],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "2df97883-b7a9-4e0d-b4f9-ff6d07314b3a".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T09:00:00Z".to_owned(),
                                "2020-07-04T11:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 22.0, lng: 0.0 },
                            duration: 15.0,
                            tag: None,
                        }),
                        delivery: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T09:00:00Z".to_owned(),
                                "2020-07-04T11:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 72.0, lng: 0.0 },
                            duration: 15.0,
                            tag: None,
                        }),
                    },
                    demand: vec![3],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "6897aeb6-e261-404b-b715-0b47c215316a".to_owned(),
                    places: JobPlaces {
                        pickup: None,
                        delivery: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T16:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 68.0, lng: 0.0 },
                            duration: 14.0,
                            tag: None,
                        }),
                    },
                    demand: vec![4],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "a8ec103c-8c09-4068-8d72-7e8bdc2eff88".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![
                                vec!["2020-07-04T09:00:00Z".to_owned(), "2020-07-04T11:00:00Z".to_owned()],
                                vec!["2020-07-04T14:00:00Z".to_owned(), "2020-07-04T16:00:00Z".to_owned()],
                            ]),
                            location: Location { lat: 96.0, lng: 0.0 },
                            duration: 14.0,
                            tag: None,
                        }),
                        delivery: Some(JobPlace {
                            times: Some(vec![
                                vec!["2020-07-04T09:00:00Z".to_owned(), "2020-07-04T11:00:00Z".to_owned()],
                                vec!["2020-07-04T14:00:00Z".to_owned(), "2020-07-04T16:00:00Z".to_owned()],
                            ]),
                            location: Location { lat: 68.0, lng: 0.0 },
                            duration: 14.0,
                            tag: None,
                        }),
                    },
                    demand: vec![3],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "a3c52d4a-d57e-4b70-b801-5178f918a430".to_owned(),
                    places: JobPlaces {
                        pickup: None,
                        delivery: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T16:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 44.0, lng: 0.0 },
                            duration: 11.0,
                            tag: None,
                        }),
                    },
                    demand: vec![4],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "5a63db05-9e00-4410-b07c-589ab173342e".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![
                                vec!["2020-07-04T09:00:00Z".to_owned(), "2020-07-04T13:00:00Z".to_owned()],
                                vec!["2020-07-04T14:00:00Z".to_owned(), "2020-07-04T16:00:00Z".to_owned()],
                            ]),
                            location: Location { lat: 36.0, lng: 0.0 },
                            duration: 17.0,
                            tag: None,
                        }),
                        delivery: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T09:00:00Z".to_owned(),
                                "2020-07-04T11:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 70.0, lng: 0.0 },
                            duration: 11.0,
                            tag: None,
                        }),
                    },
                    demand: vec![2],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "626d258d-4c06-45a8-a625-39543dcdf271".to_owned(),
                    places: JobPlaces {
                        pickup: None,
                        delivery: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T09:00:00Z".to_owned(),
                                "2020-07-04T11:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 15.0, lng: 0.0 },
                            duration: 15.0,
                            tag: None,
                        }),
                    },
                    demand: vec![3],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "a49c1f7a-eaf7-4835-afe0-27463884abb5".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T09:00:00Z".to_owned(),
                                "2020-07-04T13:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 31.0, lng: 0.0 },
                            duration: 13.0,
                            tag: None,
                        }),
                        delivery: None,
                    },
                    demand: vec![3],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "2f203228-a2c4-4494-9870-22a82131334e".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T16:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 30.0, lng: 0.0 },
                            duration: 11.0,
                            tag: None,
                        }),
                        delivery: None,
                    },
                    demand: vec![1],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "1caec13a-4c9d-4d99-a84d-c3e391dd2c19".to_owned(),
                    places: JobPlaces {
                        pickup: None,
                        delivery: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T16:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 39.0, lng: 0.0 },
                            duration: 14.0,
                            tag: None,
                        }),
                    },
                    demand: vec![1],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "5ee6c2e3-5e41-4b00-9bc1-d848182cf80f".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T18:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 60.0, lng: 0.0 },
                            duration: 17.0,
                            tag: None,
                        }),
                        delivery: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T09:00:00Z".to_owned(),
                                "2020-07-04T13:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 17.0, lng: 0.0 },
                            duration: 14.0,
                            tag: None,
                        }),
                    },
                    demand: vec![3],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "de7ace77-e770-4da0-98b1-2e99b500ed0f".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T09:00:00Z".to_owned(),
                                "2020-07-04T11:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 72.0, lng: 0.0 },
                            duration: 13.0,
                            tag: None,
                        }),
                        delivery: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T16:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 36.0, lng: 0.0 },
                            duration: 10.0,
                            tag: None,
                        }),
                    },
                    demand: vec![1],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "eb33da29-33b7-481f-afbb-dd59f76f1606".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![
                                vec!["2020-07-04T09:00:00Z".to_owned(), "2020-07-04T13:00:00Z".to_owned()],
                                vec!["2020-07-04T14:00:00Z".to_owned(), "2020-07-04T16:00:00Z".to_owned()],
                            ]),
                            location: Location { lat: 37.0, lng: 0.0 },
                            duration: 12.0,
                            tag: None,
                        }),
                        delivery: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T09:00:00Z".to_owned(),
                                "2020-07-04T13:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 2.0, lng: 0.0 },
                            duration: 10.0,
                            tag: None,
                        }),
                    },
                    demand: vec![3],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "1f98ae25-d3e0-42de-a36c-4e7e0c1cd1fd".to_owned(),
                    places: JobPlaces {
                        pickup: None,
                        delivery: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T09:00:00Z".to_owned(),
                                "2020-07-04T13:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 98.0, lng: 0.0 },
                            duration: 18.0,
                            tag: None,
                        }),
                    },
                    demand: vec![1],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "80e13f9b-33b2-4cf8-bfc9-31356bc17d93".to_owned(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T16:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 65.0, lng: 0.0 },
                            duration: 17.0,
                            tag: None,
                        }),
                        delivery: None,
                    },
                    demand: vec![4],
                    skills: None,
                }),
            ],
            relations: None,
        },
        fleet: Fleet {
            types: vec![VehicleType {
                id: "c096634a-0055-4f7a-b7fc-6aa8420969b0".to_owned(),
                profile: "car".to_owned(),
                costs: VehicleCosts { fixed: Some(30.0), distance: 0.0015, time: 0.005 },
                shifts: vec![VehicleShift {
                    start: VehiclePlace {
                        time: "2020-07-04T09:00:00Z".to_owned(),
                        location: Location { lat: 0.0, lng: 0.0 },
                    },
                    end: Some(VehiclePlace {
                        time: "2020-07-04T18:00:00Z".to_owned(),
                        location: Location { lat: 0.0, lng: 0.0 },
                    }),
                    breaks: Some(vec![VehicleBreak {
                        times: VehicleBreakTime::TimeWindows(vec![vec![
                            "2020-07-04T12:00:00Z".to_owned(),
                            "2020-07-04T14:00:00Z".to_owned(),
                        ]]),
                        duration: 3600.0,
                        location: None,
                    }]),
                    reloads: Some(vec![
                        JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T14:00:00Z".to_owned(),
                                "2020-07-04T18:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 0.0, lng: 0.0 },
                            duration: 1899.0,
                            tag: None,
                        },
                        JobPlace {
                            times: Some(vec![vec![
                                "2020-07-04T09:00:00Z".to_owned(),
                                "2020-07-04T13:00:00Z".to_owned(),
                            ]]),
                            location: Location { lat: 0.0, lng: 0.0 },
                            duration: 2260.0,
                            tag: None,
                        },
                    ]),
                }],
                capacity: vec![47],
                amount: 2,
                skills: None,
                limits: None,
            }],
            profiles: vec![Profile { name: "car".to_owned(), profile_type: "car".to_owned() }],
        },
        config: None,
    };

    let result = solve_and_check(problem);

    assert_eq!(result, Ok(()));
}
