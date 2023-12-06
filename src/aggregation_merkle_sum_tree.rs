use halo2_proofs::halo2curves::bn256::Fr as Fp;
use num_bigint::BigUint;
use std::error::Error;
use summa_backend::merkle_sum_tree::utils::{build_merkle_tree_from_leaves, fp_to_big_uint};
use summa_backend::merkle_sum_tree::{
    Cryptocurrency, Entry, MerkleProof, MerkleSumTree, Node, Tree,
};

/// Aggregation Merkle Sum Tree Data Structure.
///
/// Starting from a set of "mini" Merkle Sum Trees of equal depth, N_CURRENCIES and N_BYTES, the Aggregation Merkle Sum Tree inherits the properties of a Merkle Sum Tree and adds the following:
/// * Each Leaf of the Aggregation Merkle Sum Tree is the root of a "mini" Merkle Sum Tree made of `hash` and `balances`
///
/// # Type Parameters
///
/// * `N_CURRENCIES`: The number of assets for each user account
/// * `N_BYTES`: Range in which each node balance should lie
#[derive(Debug, Clone)]
pub struct AggregationMerkleSumTree<const N_CURRENCIES: usize, const N_BYTES: usize> {
    root: Node<N_CURRENCIES>,
    nodes: Vec<Vec<Node<N_CURRENCIES>>>,
    depth: usize,
    cryptocurrencies: Vec<Cryptocurrency>,
    mini_trees: Vec<MerkleSumTree<N_CURRENCIES, N_BYTES>>,
}

impl<const N_CURRENCIES: usize, const N_BYTES: usize> Tree<N_CURRENCIES, N_BYTES>
    for AggregationMerkleSumTree<N_CURRENCIES, N_BYTES>
{
    fn root(&self) -> &Node<N_CURRENCIES> {
        &self.root
    }

    fn depth(&self) -> &usize {
        &self.depth
    }

    fn leaves(&self) -> &[Node<N_CURRENCIES>] {
        &self.nodes[0]
    }

    fn nodes(&self) -> &[Vec<Node<N_CURRENCIES>>] {
        &self.nodes
    }

    fn cryptocurrencies(&self) -> &[Cryptocurrency] {
        &self.cryptocurrencies
    }

    fn entries(&self) -> &[Entry<N_CURRENCIES>] {
        self.mini_trees[0].entries()
    }

    fn get_entry(&self, user_index: usize) -> &Entry<N_CURRENCIES> {
        let (mini_tree_index, entry_index) = self.get_entry_location(user_index);

        // Retrieve the mini tree
        let mini_tree = &self.mini_trees[mini_tree_index];

        // Retrieve the entry within the mini tree
        mini_tree.get_entry(entry_index)
    }

    fn generate_proof(
        &self,
        index: usize,
    ) -> Result<MerkleProof<N_CURRENCIES, N_BYTES>, Box<dyn Error>>
    where
        [usize; N_CURRENCIES + 1]: Sized,
        [usize; N_CURRENCIES + 2]: Sized,
    {
        let (mini_tree_index, entry_index) = self.get_entry_location(index);

        // Retrieve the mini tree
        let mini_tree = &self.mini_trees[mini_tree_index];

        // Retrieve sibling mini tree
        let sibling_mini_tree_index = if mini_tree_index % 2 == 0 {
            mini_tree_index + 1
        } else {
            mini_tree_index - 1
        };
        let sibling_mini_tree = &self.mini_trees[sibling_mini_tree_index];

        // Build the partial proof, namely from the leaf to the root of the mini tree
        let mut partial_proof = mini_tree.generate_proof(entry_index)?;
        let mut sibling_middle_node_hash_preimages = Vec::new();

        // Retrieve sibling mini tree root hash preimage
        let sibling_mini_tree_node_preimage = sibling_mini_tree
            .get_middle_node_hash_preimage(*sibling_mini_tree.depth(), 0)
            .unwrap();

        sibling_middle_node_hash_preimages.push(sibling_mini_tree_node_preimage);

        // Build the rest of the proof (top_proof), namely from the root of the mini tree to the root of the aggregation tree
        let mut current_index = mini_tree_index;
        let mut path_indices = vec![Fp::from(0); self.depth];

        #[allow(clippy::needless_range_loop)]
        for level in 0..self.depth {
            let position = current_index % 2;
            path_indices[level] = Fp::from(position as u64);

            let sibling_index = current_index - position + (1 - position);
            if sibling_index < self.nodes[level].len() && level != 0 {
                // Fetch hash preimage for sibling middle nodes
                let sibling_node_preimage =
                    self.get_middle_node_hash_preimage(level, sibling_index)?;
                sibling_middle_node_hash_preimages.push(sibling_node_preimage);
            }
            current_index /= 2;
        }

        partial_proof.path_indices.extend(path_indices);
        partial_proof
            .sibling_middle_node_hash_preimages
            .extend(sibling_middle_node_hash_preimages);

        // replace the root of the partial proof with the root of the aggregation tree
        partial_proof.root = self.root.clone();

        Ok(partial_proof)
    }
}

impl<const N_CURRENCIES: usize, const N_BYTES: usize>
    AggregationMerkleSumTree<N_CURRENCIES, N_BYTES>
{
    /// Builds a AggregationMerkleSumTree from a set of mini MerkleSumTrees
    /// The leaves of the AggregationMerkleSumTree are the roots of the mini MerkleSumTrees
    pub fn new(
        mini_trees: Vec<MerkleSumTree<N_CURRENCIES, N_BYTES>>,
        cryptocurrencies: Vec<Cryptocurrency>,
    ) -> Result<AggregationMerkleSumTree<N_CURRENCIES, N_BYTES>, Box<dyn std::error::Error>>
    where
        [usize; N_CURRENCIES + 1]: Sized,
        [usize; N_CURRENCIES + 2]: Sized,
    {
        if mini_trees.is_empty() {
            return Err("Empty mini tree inputs".into());
        }

        // assert that all mini trees have the same depth
        let depth = mini_trees[0].depth();
        assert!(mini_trees.iter().all(|x| x.depth() == depth));

        // extract all the roots of the mini trees
        let roots = mini_trees
            .iter()
            .map(|x| x.root().clone())
            .collect::<Vec<Node<N_CURRENCIES>>>();

        let depth = (roots.len() as f64).log2().ceil() as usize;

        // Calculate the accumulated balances for each asset
        let mut balances_acc: Vec<Fp> = vec![Fp::from(0); N_CURRENCIES];

        for root in &roots {
            for (i, balance) in root.balances.iter().enumerate() {
                balances_acc[i] += *balance;
            }
        }

        // Iterate through the balance accumulator and throw error if any balance is not in range 0, 2 ^ (8 * N_BYTES):
        for balance in &balances_acc {
            // transform the balance to a BigUint
            let balance_big_uint = fp_to_big_uint(*balance);

            if balance_big_uint >= BigUint::from(2_usize).pow(8 * N_BYTES as u32) {
                return Err(
                    "Accumulated balance is not in the expected range, proof generation will fail!"
                        .into(),
                );
            }
        }

        let mut nodes = vec![];
        let root = build_merkle_tree_from_leaves(&roots, depth, &mut nodes)?;

        Ok(AggregationMerkleSumTree {
            root,
            nodes,
            depth,
            cryptocurrencies,
            mini_trees,
        })
    }

    pub fn mini_tree(&self, tree_index: usize) -> &MerkleSumTree<N_CURRENCIES, N_BYTES> {
        &self.mini_trees[tree_index]
    }

    /// starting from a user_index, returns the index of the mini tree in which the entry is located and the index of the entry within the mini tree
    fn get_entry_location(&self, user_index: usize) -> (usize, usize) {
        let entries_per_mini_tree = 1 << self.mini_trees[0].depth();

        // Calculate which mini tree the entry is in
        let mini_tree_index = user_index / entries_per_mini_tree;

        // Calculate the index within the mini tree
        let entry_index = user_index % entries_per_mini_tree;

        (mini_tree_index, entry_index)
    }
}

#[cfg(test)]
mod test {
    use summa_backend::merkle_sum_tree::{MerkleSumTree, Tree};

    use crate::aggregation_merkle_sum_tree::AggregationMerkleSumTree;

    const N_CURRENCIES: usize = 2;
    const N_BYTES: usize = 8;

    #[test]
    fn test_aggregation_mst() {
        // create new mini merkle sum tree
        let mini_tree_1 =
            MerkleSumTree::<N_CURRENCIES, N_BYTES>::from_csv("src/orchestrator/csv/entry_16_1.csv")
                .unwrap();

        let mini_tree_2 =
            MerkleSumTree::<N_CURRENCIES, N_BYTES>::from_csv("src/orchestrator/csv/entry_16_2.csv")
                .unwrap();

        let aggregation_mst = AggregationMerkleSumTree::<N_CURRENCIES, N_BYTES>::new(
            vec![mini_tree_1.clone(), mini_tree_2.clone()],
            mini_tree_1.cryptocurrencies().to_owned().to_vec(),
        )
        .unwrap();

        // get root
        let root = aggregation_mst.root();

        // expect root hash to be different than 0
        assert!(root.hash != 0.into());
        // expect balance to match the sum of all entries
        assert!(root.balances == [(556862 * 2).into(), (556862 * 2).into()]);

        // expect depth to be equal to merkle_sum_tree_1.depth (= merkle_sum_tree_2.depth) + 1
        let depth = aggregation_mst.depth();

        assert!(*depth == 1);

        let mut index = rand::random::<usize>() % 32;

        // the entry fetched from the aggregation tree should be the same as the entry fetched from the corresponding mini tree
        let entry = aggregation_mst.get_entry(index);

        if index < 16 {
            assert!(entry.username() == mini_tree_1.get_entry(index).username());
            assert!(entry.balances() == mini_tree_1.get_entry(index).balances());
        } else {
            index -= 16;
            assert!(entry.username() == mini_tree_2.get_entry(index).username());
            assert!(entry.balances() == mini_tree_2.get_entry(index).balances());
        }

        // Generate proof for the entry
        let proof = aggregation_mst.generate_proof(index).unwrap();

        // verify proof
        assert!(aggregation_mst.verify_proof(&proof));
    }

    #[test]
    fn test_aggregation_mst_compare_mst_result() {
        // create new mini merkle sum tree
        let mut mini_trees = Vec::new();
        for i in 1..=4 {
            let mini_tree = MerkleSumTree::<N_CURRENCIES, N_BYTES>::from_csv(&format!(
                "src/orchestrator/csv/entry_16_{}.csv",
                i
            ))
            .unwrap();
            mini_trees.push(mini_tree);
        }
        let cryptocurrencies = mini_trees[0].cryptocurrencies().to_owned().to_vec();
        let aggregation_mst =
            AggregationMerkleSumTree::<N_CURRENCIES, N_BYTES>::new(mini_trees, cryptocurrencies)
                .unwrap();

        let aggregation_mst_root = aggregation_mst.root();

        // The entry_64.csv file is the aggregation of entry_16_1, entry_16_2, entry_16_3, entry_16_4
        let single_merkle_sum_tree =
            MerkleSumTree::<N_CURRENCIES, N_BYTES>::from_csv("src/orchestrator/csv/entry_64.csv")
                .unwrap();

        assert_eq!(
            aggregation_mst_root.hash,
            single_merkle_sum_tree.root().hash
        );
    }

    #[test]
    fn test_aggregation_mst_overflow() {
        // create new mini merkle sum trees. The accumulated balance for each mini tree is in the expected range
        // note that the accumulated balance of the tree generated from entry_16_4 is just in the expected range for 1 unit
        let merkle_sum_tree_1 =
            MerkleSumTree::<N_CURRENCIES, N_BYTES>::from_csv("src/orchestrator/csv/entry_16.csv")
                .unwrap();

        let merkle_sum_tree_2 = MerkleSumTree::<N_CURRENCIES, N_BYTES>::from_csv(
            "src/orchestrator/csv/entry_16_no_overflow.csv",
        )
        .unwrap();

        // When creating the aggregation merkle sum tree, the accumulated balance of the two mini trees is not in the expected range, an error is thrown
        let result = AggregationMerkleSumTree::<N_CURRENCIES, N_BYTES>::new(
            vec![merkle_sum_tree_1, merkle_sum_tree_2.clone()],
            merkle_sum_tree_2.cryptocurrencies().to_vec(),
        );

        if let Err(e) = result {
            assert_eq!(
                e.to_string(),
                "Accumulated balance is not in the expected range, proof generation will fail!"
            );
        }
    }
}
