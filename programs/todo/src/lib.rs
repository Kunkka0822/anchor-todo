use anchor_lang::prelude::*;

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
        ctx: Context<NewList>,
        _list_name: String,
        item_name: String,
        bounty: u64,
    ) -> ProgramResult {
        let user = &ctx.accounts.user;
        let list = &mut ctx.accounts.list;
        let item = &mut ctx.accounts.item;

        if list.lines.len() >= list.capacity as usize {
            return Err(TodoListError::ListFull.into());
        }

        list.lines.push(*item.to_account_info().key);
        item.name = item_name;
        item.creator = *user.to_account_infos().key;

        let account_lamports = **item.to_account_info.account_lamports.borrow();
        let transfer_amount = bounty
            .checked_sub(account_lamports)
            .ok_or(TodoListError::BountyTooSmall)?;

        if transfer_amount > 0 {
            invoke(
                &transfer(
                    user.to_account_info().key,
                    item.to_account_info().key,
                    transfer_amount,
                ),
                &[
                    user.to_account_info(),
                    item.to_account_info(),
                    ctx.accounts.system_program.to_account_info(),
                ],
            )?;
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
            list_owner.to_account_info.key().as_ref(),
            name_seed(&list_name)
        ], bump)]
    pub list: Account<'info, TodoList>,
    pub list_owner: AccountInfo<'info>,

    #[account(init, payer=user, space=ListItem::space(&item_name))]
    pub item: Account<'info, ListItem>,
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