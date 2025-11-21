use anchore lang::prelude::*;



declare_id!("4QxPT9giJh5BxERUm59LTEpENbJh6mk1xnonBCqVMzJo");


#[program]
pub mod my_counter {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let counter = &mut ctx.accounts.counter;
        counter.count = 0;
    }

    pub fn increment(ctx: Context<Increment>) -> Result<()> {
        let counter  = &mut ctx.accounts.counter;
        counter.count += 1;
        OK(())
    }
}




#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, payer = user, space = 8 + 8)]
    pub counter: Account<'info, Counter>,

    #[account(mut)]
    pub user: Signer<'info>,

    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
pub struct Increment<'info> {
    #[aacount(mut)]
    pub counter: Account<'info, Counter>
}

#[account]
pub struct Counter{
    pub count: u64
}