use graph_builder::prelude::*;
use std::collections::HashMap;

#[derive(Copy, Clone, Debug, Default)]
pub struct Metric(pub f64, pub f64);

#[derive(Copy, Clone)]
pub struct Edge<M> {
    pub metric: M,
    pub edge_type: EdgeType,
}
impl<M> Edge<M> {
    pub fn new_decision(metric: M) -> Self {
        Self {
            metric,
            edge_type: EdgeType::Decision(1),
        }
    }
    pub fn new_repeatable_decision(metric: M, repeats: u32) -> Self {
        Self {
            metric,
            edge_type: EdgeType::Decision(repeats),
        }
    }
    pub fn new_probability(metric: M, weight: f64) -> Self {
        Self {
            metric,
            edge_type: EdgeType::Probability(weight),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum EdgeType {
    Decision(u32),
    Probability(f64),
}

// State is the generated value of the node
#[derive(Clone, Debug, PartialEq)]
pub struct State<T> {
    pub fx: (f64, f64),
    pub successor: Option<T>,
}

fn step(
    graph: &DirectedCsrGraph<usize, (), Edge<Metric>>,
    x: usize,
    x0: f64,
    state: &mut HashMap<usize, State<usize>>,
) -> State<usize> {
    if let Some(ret) = state.get(&x) {
        return ret.clone();
    }
    if graph.out_degree(x) == 0 {
        // leaf
        // node themselves have no metric, only edges
        let s = State {
            fx: (0.0, 0.0),
            successor: None,
        };
        state.insert(x, s.clone());
        return s;
    }
    // println!("step({}, {})", x, x0);
    let neighbours = graph.out_neighbors_with_values(x).collect::<Vec<_>>();
    let edge_type = neighbours[0].value.edge_type;

    // println!("step({}, {}) = {:?}", x, x0, ret);
    let mut successor = None;
    let (ret_x, ret_t) = match edge_type {
        EdgeType::Decision(_) => {
            let mut max = (f64::MIN, f64::MIN);
            for &t in neighbours.iter() {
                let y = t.target;
                let edge = t.value.metric;
                let repeats = match t.value.edge_type {
                    EdgeType::Decision(repeats) => repeats,
                    _ => panic!(),
                } as f64;
                let (g, dg) = step(graph, y, x0, state).fx;
                let f = repeats * g + (edge.0 - x0 * edge.1);
                let df = repeats * dg - edge.1;
                if f > max.0 || f == max.0 && df > max.1 {
                    max.0 = f;
                    max.1 = df;
                    successor = Some(y);
                }
            }
            max
        }
        EdgeType::Probability(_) => {
            let mut sum = (0.0, 0.0);
            let mut weight_sum = 0.0;
            for &t in neighbours.iter() {
                let y = t.target;
                let edge = t.value.metric;
                let edge_weight = match t.value.edge_type {
                    EdgeType::Probability(w) => w,
                    _ => panic!(),
                };
                let (g, dg) = step(graph, y, x0, state).fx;
                let f = g + (edge.0 - x0 * edge.1);
                let df = dg - edge.1;
                sum.0 += f * edge_weight;
                sum.1 += df * edge_weight;
                weight_sum += edge_weight;
            }
            (sum.0 / weight_sum, sum.1 / weight_sum)
        }
    };
    let s = State {
        fx: (ret_x, ret_t),
        successor,
    };
    state.insert(x, s.clone());
    s
}

pub fn evaluate(
    graph: &DirectedCsrGraph<usize, (), Edge<Metric>>,
    start_idx: usize,
) -> (f64, HashMap<usize, State<usize>>) {
    let mut x0 = 0.0;
    let mut iterations = 0;
    loop {
        let mut state: HashMap<usize, State<usize>> = HashMap::new();
        let (f, df) = step(graph, start_idx, x0, &mut state).fx;
        iterations += 1;

        // println!("f({}) = {}, df = {}", x0, f, df);
        x0 -= f / df;

        if f.abs() < 1e-6 || iterations >= 10 {
            // println!("x0 = {}", x0);
            return (x0, state);
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn graph0() {
        let mut edges: Vec<(usize, usize, Edge<Metric>)> = vec![];
        edges.push((0, 1, Edge::new_decision(Metric(1.0, 1.0))));
        edges.push((0, 2, Edge::new_decision(Metric(3.0, 2.0))));

        let graph: DirectedCsrGraph<usize, (), Edge<Metric>> =
            GraphBuilder::new().edges_with_values(edges).build();

        evaluate(&graph, 0);
    }

    #[test]
    fn graph3() {
        let mut edges: Vec<(usize, usize, Edge<Metric>)> = vec![];
        edges.push((0, 1, Edge::new_probability(Metric(0.0, 0.0), 1.0)));
        edges.push((0, 2, Edge::new_probability(Metric(0.0, 0.0), 3.0)));
        edges.push((1, 3, Edge::new_decision(Metric(10.0, 10.0))));
        edges.push((1, 4, Edge::new_decision(Metric(99.0, 100.0))));
        edges.push((2, 5, Edge::new_decision(Metric(2.1, 10.0))));
        edges.push((2, 6, Edge::new_decision(Metric(20.0, 100.0))));

        let graph: DirectedCsrGraph<usize, (), Edge<Metric>> =
            GraphBuilder::new().edges_with_values(edges).build();

        evaluate(&graph, 0);
    }

    #[test]
    fn graph1() {
        let mut edges: Vec<(usize, usize, Edge<Metric>)> = vec![];
        edges.push((0, 1, Edge::new_decision(Metric(1.0, 1.0))));
        edges.push((0, 2, Edge::new_decision(Metric(3.0, 2.0))));
        edges.push((0, 3, Edge::new_decision(Metric(2.0, 3.0))));
        edges.push((0, 4, Edge::new_decision(Metric(4.0, 3.0))));
        edges.push((0, 5, Edge::new_decision(Metric(4.0, 3.0))));
        edges.push((5, 6, Edge::new_decision(Metric(1.0, 1.0))));
        edges.push((5, 7, Edge::new_decision(Metric(1.0, 1.0))));

        let graph: DirectedCsrGraph<usize, (), Edge<Metric>> =
            GraphBuilder::new().edges_with_values(edges).build();

        evaluate(&graph, 0);
    }

    #[test]
    fn graph2() {
        let mut edges: Vec<(usize, usize, Edge<Metric>)> = vec![];
        edges.push((0, 1, Edge::new_probability(Metric(1.0, 1.0), 1.0)));
        edges.push((0, 2, Edge::new_probability(Metric(3.0, 2.0), 1.0)));
        edges.push((0, 3, Edge::new_probability(Metric(2.0, 3.0), 1.0)));
        edges.push((0, 4, Edge::new_probability(Metric(4.0, 3.0), 1.0)));

        let graph: DirectedCsrGraph<usize, (), Edge<Metric>> =
            GraphBuilder::new().edges_with_values(edges).build();

        evaluate(&graph, 0);
    }
}
