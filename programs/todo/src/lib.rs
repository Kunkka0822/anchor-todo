use std::io::Write;

use anchor_lang::{ __private::CLOSED_ACCOUNT_DISCRIMINATOR, prelude::* };

declare_id!("2NW2t7NuhrzpscaZomaEjYW2he5P9AwAnSNHbT3UEEHJ");


#[program]
pub mod todo {
    use anchor_lang::system_program::{Transfer, transfer};

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
            return Err(error!(TodoListError::ListFull));
        }

        list.lines.push(*item.to_account_info().key);
        item.name = item_name;
        item.creator = *user.to_account_info().key;

        let account_lamports = **item.to_account_info().lamports.borrow();
        let transfer_amount = bounty
            .checked_sub(account_lamports)
            .ok_or(TodoListError::BountyTooSmall)?;

        if transfer_amount > 0 {
            let cpi_ctx = CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                Transfer {
                    from: user.to_account_info(),
                    to: item.to_account_info()
                },
            );
            transfer(cpi_ctx, transfer_amount)?;
        }

        Ok(())
    }

    pub fn cancel(ctx: Context<Cancel>, _list_name: String) -> Result<()> {
        let list = &mut ctx.accounts.list;
        let item = &mut ctx.accounts.item;
        let item_creator = &ctx.accounts.item_creator;

        let user = ctx.accounts.user.to_account_info().key;

        if &list.list_owner != user && &item.creator != user {
            return Err(error!(TodoListError::CancelPermissions));
        }

        if !list.lines.contains(item.to_account_info().key) {
            return Err(error!(TodoListError::ItemNotFound));
        }

        //item.to_account_info().close(item_creator.to_account_info())?;
        close_account(
            &mut item.to_account_info(), 
            &mut item_creator.to_account_info(),
        );

        let item_key = ctx.accounts.item.to_account_info().key;
        list.lines.retain(|key| key != item_key);

        Ok(())
    }
}

pub fn close_account(
    pda_to_close: &mut AccountInfo,
    sol_destination: &mut AccountInfo,
) -> Result<()> {
    let dest_starting_lamports = sol_destination.lamports();
    **sol_destination.lamports.borrow_mut() = 
        dest_starting_lamports.try_add(pda_to_close.lamports());
    **pda_to_close.lamports.borrow_mut() = 0;

    let mut data = pda_to_close.try_borrow_data()?;
    let dst: &mut [u8] = &mut data;
    let mut cursor = std::io::Cursor::new(dst);
    cursor
        .write_all(&CLOSED_ACCOUNT_DISCRIMINATOR)
        .map_err(|_| error!(TodoListError::CloseFailed))?;
    Ok(())
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
    /// CHECK:
    pub list_owner: AccountInfo<'info>,

    #[account(init, payer=user, space=ListItem::space(&item_name))]
    pub item: Account<'info, ListItem>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(list_name: String)]
pub struct Cancel<'info> {
    #[account(mut,
        has_one=list_owner @ TodoListError::WrongListOwner,
        seeds=[
            b"todolist",
            list_owner.to_account_info().key().as_ref(),
            name_seed(&list_name)
        ], bump)]
    pub list: Account<'info, TodoList>,
    /// CHECK:
    pub list_owner: AccountInfo<'info>,
    #[account(mut)]
    pub item: Account<'info, ListItem>,
    #[account(mut, address=item.creator @ TodoListError::WrongItemCreator)]
    pub item_creator: AccountInfo<'info>,
    pub user: Signer<'info>,
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
    #[msg("Failed to close account descriminator")]
    CloseFailed,
}