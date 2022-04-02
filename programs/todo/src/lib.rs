use anchor_lang::prelude::*;
use anchor_lang::system_program::transfer;

declare_id!("2NW2t7NuhrzpscaZomaEjYW2he5P9AwAnSNHbT3UEEHJ");


#[program]
pub mod todo {
    use super::*;

    pub fn new_list(
        ctx: Context<NewList>,
        name: String,
        capacity: u16,
        account_bump: u8,
    ) -> Result<()> {
        let list = &mut ctx.accounts.list;
        list.list_owner = *ctx.accounts.user.to_account_info().key;
        list.bump = account_bump;
        list.name = name;
        list.capacity = capacity;
        Ok(())
    }

    pub fn add(
        ctx: Context<Add>,
        _list_name: String,
        item_name: String,
        bounty: u64,
    ) -> Result<()> {
        let user = &ctx.accounts.user;
        let list = &mut ctx.accounts.list;
        let item = &mut ctx.accounts.item;

        if list.lines.len() >= list.capacity as usize {
            return Err(TodoListError::ListFull.into());
        }

        list.lines.push(*item.to_account_info().key);
        item.name = item_name;
        item.creator = *user.to_account_info().key;

        let account_lamports = **item.to_account_info().lamports.borrow();
        let transfer_amount = bounty
            .checked_sub(account_lamports)
            .ok_or(TodoListError::BountyTooSmall.into())?;

        if transfer_amount > 0 {
            let cpi_ctx = CpiContext::new(
                ctx.accounts.token_program.to_account_info().clone(),
                anchor_spl::token::Transfer {
                    from: ctx.accounts.reward_mint_account.to_account_info().clone(),
                    to: ctx.accounts.reward_account.to_account_info().clone(),
                    authority: ctx.accounts.reward.to_account_info().clone(),
                },
            );
            anchor_spl::token::transfer(cpi_ctx.with_signer(&[&seeds[..]]), reward_amount)?;
            // invoke(
            //     &transfer(
            //         user.to_account_info().key,
            //         item.to_account_info().key,
            //         transfer_amount,
            //     ),
            //     &[
            //         user.to_account_info(),
            //         item.to_account_info(),
            //         ctx.accounts.system_program.to_account_info(),
            //     ],
            // )?;
        }

        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(name: String, capacity: u16, list_bump: u8)]
pub struct NewList<'info> {
    #[account(init, payer=user,
        space=TodoList::space(&name, capacity),
        seeds=[
            b"todolist",
            user.to_account_info().key().as_ref(),
            name_seed(&name)
        ],
        bump)]
    pub list: Account<'info, TodoList>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(list_name: String, item_name: String, bounty: u64)]
pub struct Add<'info> {
    #[account(
        mut,
        has_one=list_owner @ TodoListError::WrongListOwner,
        seeds=[
            b"todolist",
            list_owner.to_account_info().key().as_ref(),
            name_seed(&list_name)
        ], bump)]
    pub list: Account<'info, TodoList>,
    pub list_owner: AccountInfo<'info>,

    #[account(init, payer=user, space=ListItem::space(&item_name))]
    pub item: Account<'info, ListItem>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[account]
pub struct TodoList {
    pub list_owner: Pubkey,
    pub bump: u8,
    pub capacity: u16,
    pub name: String,
    pub lines: Vec<Pubkey>,
}

impl TodoList {
    fn space(name: &str, capacity: u16) -> usize {
        // discriminator + owner pubkey + bump + capacity
        8 + 32 + 1 + 2 + 
        // name string
        4 + name.len() +
        // vec of item pubkeys
        4 + (capacity as usize) * std::mem::size_of::<Pubkey>()
    }
}

#[account]
pub struct ListItem {
    pub creator: Pubkey,
    pub creator_finished: bool,
    pub list_owner_finished: bool,
    pub name: String,
}

impl ListItem {
    fn space(name: &str) -> usize {
        //discriminator + creator pubkey + 2 bools + name string
        8 + 32 + 2 + 4 + name.len()
    }
}



fn name_seed(name: &str) -> &[u8] {
    let b = name.as_bytes();
    if b.len() > 32 { &b[0..32] } else { b }
}

#[error_code]
pub enum TodoListError {
    #[msg("This list is full")]
    ListFull,
    #[msg("Bounty must be enough to mark account rent-exempt")]
    BountyTooSmall,
    #[msg("Only the list owner or item creator may cancel an item")]
    CancelPermissions,
    #[msg("Only the list owner or item creator may finish an item")]
    FinishPermissions,
    #[msg("Item does not belong to this todo list")]
    ItemNotFound,
    #[msg("Specified list owner does not match the pubkey in the list")]
    WrongListOwner,
    #[msg("Specified item creator does not match the pubkey in the item")]
    WrongItemCreator,
}