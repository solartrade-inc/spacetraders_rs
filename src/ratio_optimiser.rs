use graph_builder::prelude::*;
use std::ops::Add;

#[derive(Copy, Clone, Debug, Default)]
struct Metric(f64, f64);

impl Add for Metric {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self(self.0 + other.0, self.1 + other.1)
    }
}

struct Vertex {}

#[derive(Copy, Clone)]
struct Edge<M> {
    metric: M,
    edge_type: EdgeType,
}
impl<M> Edge<M> {
    fn new_decision(metric: M) -> Self {
        Self {
            metric,
            edge_type: EdgeType::Decision,
        }
    }
    fn new_probability(metric: M, weight: f64) -> Self {
        Self {
            metric,
            edge_type: EdgeType::Probability(weight),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
enum EdgeType {
    Decision,
    Probability(f64),
}

// fn evaluate(graph: &DirectedCsrGraph<usize, (), Edge<Metric>>) {
//     // dfs
//     #[derive(Clone, Debug, PartialEq)]
//     enum Color {
//         Unvisited,
//         Visited,
//         Exited,
//     }
//     let mut stack: Vec<(usize, bool)> = vec![];
//     let mut color: Vec<Color> = vec![Color::Unvisited; graph.node_count()];

//     let mut sum: Vec<Metric> = vec![Metric::default(); graph.node_count()];

//     let INTERVAL_START = f64::MIN;
//     let INTERVAL_END = f64::MAX;

//     // (start, end, x, t)
//     let mut interval: Vec<(f64, f64, f64, f64)> = vec![];
//     let mut interval_idx: Vec<Option<(usize, usize)>> = vec![None; graph.node_count()];

//     stack.push((0, false));
//     while let Some((x, is_exit)) = stack.pop() {
//         match is_exit {
//             false => {
//                 if color[x] != Color::Unvisited {
//                     continue;
//                 }
//                 color[x] = Color::Visited;
//                 println!("A {}", x);
//                 stack.push((x, true));
//                 for &t in graph.out_neighbors_with_values(x).rev() {
//                     let y = t.target;
//                     let edge = t.value;
//                     // in forward pass, we sum the metric
//                     sum[y] = sum[x] + edge.metric;
//                     stack.push((y, false));
//                 }
//             }
//             true => {
//                 color[x] = Color::Exited;
//                 println!("B {}", x);
//                 assert!(interval_idx[x].is_none());

//                 if graph.out_degree(x) == 0 {
//                     // if leaf then single (complete) interval
//                     let idx = interval.len();
//                     interval.push((INTERVAL_START, INTERVAL_END, sum[x].0, sum[x].1));
//                     interval_idx[x] = Some((idx, idx));
//                 }
//                 else {
//                     // if not leaf then merge intervals
//                     let neighbors = graph.out_neighbors_with_values(x).collect::<Vec<_>>();
//                     let mut x_interval_idx = interval_idx[neighbors[0].target].unwrap();
//                     let node_type = neighbors[0].value.edge_type;
//                     println!("interval_idx[{}] = {}, {}", neighbors[0].target, x_interval_idx.0, x_interval_idx.1);
//                     for &t in graph.out_neighbors_with_values(x).skip(1) {
//                         let y = t.target;
//                         let (a, b) = interval_idx[y].unwrap();
                        
//                         println!("interval_idx[{}] = {}, {}", y, a, b);
//                         assert_eq!(x_interval_idx.1 + 1, a);
//                         assert_eq!(node_type, t.value.edge_type);
//                         println!("merging");
//                         println!("item 1:");
//                         for i in x_interval_idx.0..=x_interval_idx.1 {
//                             println!("{}: {:?}", i, interval[i]);
//                         }
//                         println!("item 2:");
//                         for i in a..=b {
//                             println!("{}: {:?}", i, interval[i]);
//                         }

//                         // merge


//                         x_interval_idx.1 = b;
//                     }
//                     println!("x_interval_idx {:?}", x_interval_idx);
//                     interval_idx[x] = Some(x_interval_idx);

//                     // actually merge
                    
//                 }
//             }
//         }

//     }

//     // todo: 
//     // we need to merge up the tree, merging leaf nodes into their parents until only the root remains
//     // the merged nodes should use similar amount of memory, but are associated with a new single node

//     // and almost done
//     // finally compute X for each interval, and pick the greatest that aligns X = F(X)

//     println!("{:?}", interval);
// }


// fn f(graph: &DirectedCsrGraph<usize, (), Edge<Metric>>) {

fn step(graph: &DirectedCsrGraph<usize, (), Edge<Metric>>, x: usize, x0: f64) -> (f64, f64) {
    if graph.out_degree(x) == 0 {
        // leaf
        // node themselves have no metric, only edges
        return (0.0, 0.0);
    }
    // println!("step({}, {})", x, x0);
    let neighbours = graph.out_neighbors_with_values(x).collect::<Vec<_>>();
    let edge_type = neighbours[0].value.edge_type;
    
    let ret = match edge_type {
        EdgeType::Decision => {
            let mut max = (f64::MIN, f64::MIN);
            for &t in neighbours.iter() {
                let y = t.target;
                let edge = t.value.metric;
                if let EdgeType::Probability(_) = t.value.edge_type {
                    panic!()
                }
                let (g, dg) = step(graph, y, x0);
                let f = g + (edge.0 - x0 * edge.1);
                let df = dg - edge.1;
                if f > max.0 || f == max.0 && df > max.1{
                    max.0 = f;
                    max.1 = df;
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
                    _ => panic!()
                };
                let (g, dg) = step(graph, y, x0);
                let f = g + (edge.0 - x0 * edge.1);
                let df = dg - edge.1;
                sum.0 += f * edge_weight;
                sum.1 += df * edge_weight;
                weight_sum += edge_weight;
            }
            (sum.0 / weight_sum, sum.1 / weight_sum)
        }
    };
    // println!("step({}, {}) = {:?}", x, x0, ret);
    ret
}

fn evaluate(graph: &DirectedCsrGraph<usize, (), Edge<Metric>>) {
    let mut x0 = 0.0;
    for _ in 0..10 {
        let (f, df) = step(graph, 0, x0);
        println!("f({}) = {}, df = {}", x0, f, df);
        x0 -= f / df;
    }
    println!("x0 = {}", x0);
}

// #[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn graph0() {
        let mut edges: Vec<(usize, usize, Edge<Metric>)> = vec![];
        edges.push((0, 1, Edge::new_decision(Metric(1.0, 1.0))));
        edges.push((0, 2, Edge::new_decision(Metric(3.0, 2.0))));

        let graph: DirectedCsrGraph<usize, (), Edge<Metric>> = GraphBuilder::new()
            .edges_with_values(edges)
            .build();

        let result = evaluate(&graph);
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
    
        let graph: DirectedCsrGraph<usize, (), Edge<Metric>> = GraphBuilder::new()
            .edges_with_values(edges)
            .build();

        let result = evaluate(&graph);
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

        let graph: DirectedCsrGraph<usize, (), Edge<Metric>> = GraphBuilder::new()
            .edges_with_values(edges)
            .build();

        let result = evaluate(&graph);
    }

    #[test]
    fn graph2() {
        let mut edges: Vec<(usize, usize, Edge<Metric>)> = vec![];
        edges.push((0, 1, Edge::new_probability(Metric(1.0, 1.0), 1.0)));
        edges.push((0, 2, Edge::new_probability(Metric(3.0, 2.0), 1.0)));
        edges.push((0, 3, Edge::new_probability(Metric(2.0, 3.0), 1.0)));
        edges.push((0, 4, Edge::new_probability(Metric(4.0, 3.0), 1.0)));

        let graph: DirectedCsrGraph<usize, (), Edge<Metric>> = GraphBuilder::new()
            .edges_with_values(edges)
            .build();

        let result = evaluate(&graph);
    }
}
