use crate::helpers::algorithms::gsom::{create_test_network, Data};
use crate::utils::{DefaultRandom, Random};

#[test]
fn can_train_network() {
    let mut network = create_test_network();
    let samples = vec![Data::new(1.0, 0.0, 0.0), Data::new(0.0, 1.0, 0.0), Data::new(0.0, 0.0, 1.0)];

    // train
    let random = DefaultRandom::default();
    for _ in 1..4 {
        for _ in 1..500 {
            let sample_i = random.uniform_int(0, samples.len() as i32 - 1) as usize;
            network.train(samples[sample_i].clone(), true);
        }

        network.retrain(10, &|node| node.read().unwrap().storage.data.is_empty());
    }

    assert!(!network.nodes.len() >= 3);
    assert_eq!(network.nodes.len(), network.size());
    samples.iter().for_each(|sample| {
        let node = network.find_bmu(sample);
        let node = node.read().unwrap();

        assert_eq!(node.storage.data.first().unwrap().values, sample.values);
        assert_eq!(node.weights.iter().map(|v| v.round()).collect::<Vec<_>>(), sample.values);
    });
}
