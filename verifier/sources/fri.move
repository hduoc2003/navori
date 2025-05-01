module verifier_addr::fri {
    use std::signer::address_of;
    use std::vector;

    friend verifier_addr::fri_statement_contract;
    friend verifier_addr::fri_layer;
    friend verifier_addr::merkle_verifier;
    friend verifier_addr::merkle_statement_contract;

    struct Fri has key, store, drop {
        fri: vector<u256>
    }

    public(friend) fun new_fri(): vector<u256> {
        let fri = vector[];
        for (i in 0..800) {
            vector::push_back(&mut fri, 0_u256);
        };
        fri
    }

    public(friend) fun get_fri(signer: address): vector<u256> acquires Fri {
        let Fri { fri } = move_from<Fri>(signer);
        fri
    }

    public(friend) fun update_fri(signer: &signer, fri: vector<u256>) acquires Fri {
        let signer_addr = address_of(signer);
        if (exists<Fri>(signer_addr)) {
            *borrow_global_mut<Fri>(signer_addr) = Fri { fri };
        } else {
            move_to(signer, Fri { fri });
        }
    }

    #[view]
    public fun view_fri(signer: address): vector<u256> acquires Fri {
        let Fri { fri } = move_from<Fri>(signer);
        fri
    }

    public entry fun reset_fri(signer: &signer) acquires Fri {
        move_from<Fri>(address_of(signer));
    }
}
