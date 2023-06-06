use super::*;

#[test]
fn test_builder() {
    let mut ssa = Ssa::default();

    let var_lhs = ssa.variable();
    let var_rhs = ssa.variable();
    let var_c = ssa.variable();

    let head = {
        let head = ssa.block();
        ssa.write(head, var_lhs, Inst::Empty);
        ssa.write(head, var_rhs, Inst::Empty);
        ssa.write(head, var_c, Inst::Empty);

        let inst = {
            let lhs = ssa.read(head, var_lhs);
            let rhs = ssa.read(head, var_rhs);
            Inst::Binary {
                op: BinOp::Add,
                lhs,
                rhs,
            }
        };
        ssa.write(head, var_c, inst);

        let inst = {
            let lhs = ssa.read(head, var_lhs);
            let rhs = ssa.read(head, var_c);
            Inst::Binary {
                op: BinOp::Add,
                lhs,
                rhs,
            }
        };
        ssa.write(head, var_c, inst);
        ssa.seal_block(head);
        head
    };

    let a = {
        let a = ssa.block();
        ssa.add_pred(a, head);

        let inst = {
            let lhs = ssa.read(a, var_lhs);
            let rhs = ssa.read(a, var_c);
            Inst::Binary {
                op: BinOp::Mul,
                lhs,
                rhs,
            }
        };
        ssa.write(a, var_c, inst);
        ssa.seal_block(a);
        a
    };

    let b = {
        let b = ssa.block();
        ssa.add_pred(b, head);
        ssa.add_pred(b, a);

        let inst = {
            let lhs = ssa.read(b, var_lhs);
            let rhs = ssa.read(b, var_c);
            Inst::Binary {
                op: BinOp::Add,
                lhs,
                rhs,
            }
        };
        ssa.write(b, var_c, inst);
        ssa.seal_block(b);
        b
    };

    println!("head:");

    for (index, inst) in ssa.instructions(head) {
        println!("  {index} = {inst}")
    }

    println!("a:");

    for (index, inst) in ssa.instructions(a) {
        println!("  {index} = {inst}")
    }

    println!("b:");

    for (index, inst) in ssa.instructions(b) {
        println!("  {index} = {inst}")
    }
}
