use std::{
    env,
    error::Error,
    collections::HashMap,
};
use csv::ReaderBuilder;
use serde::Deserialize;

//helper enum to easily identify which type of transactions we are working with
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum TransactionType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

//this struct will contain all the pertinent information surrounding a transaction we grab from each CSV row
// NOTE that the amount field is optional since not all transaction types provide an amount 
#[derive(Copy, Clone, Debug, Deserialize)]
struct Transaction {
    #[serde(alias = "type")]
    tx_type: TransactionType,
    client: u16,
    tx: u32,
    amount: Option<f32>,
}

//this struct will contain all the infromation for a particular client, and we can update their funds as transactions come in
#[derive(Copy, Clone, Debug)]
struct Account {
    available: f32,
    held: f32,
    total: f32,
    locked: bool,
} 

//helper struct to easily record the amount being disputed
#[derive(Copy, Clone, Debug)]
struct DisputeAmt(f32);

//this will help us map client id --> client account infromation. This way, when a transaction comes in, we can easily grab the cient account info
// in O(1) time then update their account
#[derive(Clone, Debug)]
struct ClientList(HashMap<u16, Account>);

//this will help us map both valid transaction id --> Transaction and valid dispute id --> dipute amount
//This way when we have a dispute, we can find its according transaction in 0(1) time 
//and when we have a resolution or chargeback of the dispute, we can grab the proper dispute amt in 0(1) time
#[derive(Clone, Debug)]
struct Transactions {
    valid: HashMap<u32, Transaction>,
    disputes: HashMap<u32, DisputeAmt>,
}
                  
fn main(){
    //collect the input strings into vector since it makes it easier to work with
    let args: Vec<String> = env::args().collect();
    //make sure that we have only passed in one arg which should be the input file
    assert!(args.len() == 2, "Only arg should be input file in the form \"cargo r -- test.csv\"");
    let input_file = &args[1];
    //istantiate our transaction and account lists that we will be updated as we parse the CSV
    let mut clients = ClientList(HashMap::new());
    let mut transactions = Transactions{ valid:  HashMap::new(), disputes: HashMap::new() };
    //helper function to prase the CSV and update our data structures accordinlgy
    match parse_csv(input_file, &mut transactions, &mut clients) {
        //if nothign went wrong with parsing the input, print the output, otherwise print the error stack
        Ok(_) => print_client_info(&clients),
        Err(error) => panic!("Problem parsing the input file: {:?}", error),
    };
}

fn parse_csv(file_name: &str, transactions: &mut Transactions, clients: &mut ClientList) -> Result<(), Box<dyn Error>> {
    //create the CSV reader that reads in from the path of the CSV file, set flexible to true since rows could be of uneven length
    let mut reader = ReaderBuilder::new().flexible(true).from_path(file_name)?;
    //create a record object to store each raw record in as we stream from the csv. Allocating only once and overwriting saves time and memory
    let mut raw_record = csv::StringRecord::new();
    //grab the headers from the csv file to make for easy deserialization 
    let mut headers = reader.headers()?.clone();
    //trim headers since we want to account for any potentail raw space
    headers.trim();
    
    //stream from the csv reader while there are still rows to process
    while reader.read_record(&mut raw_record)? {
        //trim the row to make sure whitesapce is gone  
        raw_record.trim();
        //deserialize the CSV row into a Transaction   
        let t: Transaction = raw_record.deserialize(Some(&headers))?;

        //match the trasaction type to execute the proepr logic for parsing each type
        match t.tx_type {
            TransactionType::Deposit => handle_deposit(t, transactions, clients),
            TransactionType::Withdrawal => handle_withdrawal(t, transactions, clients),
            TransactionType::Dispute => handle_dispute(t, transactions, clients),
            TransactionType::Resolve => handle_resolve(t, transactions, clients),
            TransactionType::Chargeback => handle_chargeback(t, transactions, clients)
        }
    }
    Ok(())
}

fn handle_deposit(t: Transaction, transactions: &mut Transactions, clients: &mut ClientList) {
    //check that the client account already exists, by client ID
    if let Some(client_acc) = clients.0.get_mut(&t.client) {
        //don't process any further if the client account is frozen
        if client_acc.locked { return }
        //if the client account does exist, then we update their total funds and their available funds
        //its ok to unwrap here since this function would only be called on a deposit which is garaunteed to have an amount
        client_acc.available += t.amount.unwrap();
        client_acc.total += t.amount.unwrap();
    }
    else {
        //If this is a client's first deposit, then we create an account for them and add to the accountlist
        //Their account can be accessed by their client id in the future using the hashmap
        let open_new_acc = Account {
            available: t.amount.unwrap(),
            held: 0.0,
            total: t.amount.unwrap(),
            locked: false,
        };
        clients.0.insert(t.client, open_new_acc);
    }
    //add the transaction to the valid list of transactions that was processed
    transactions.valid.insert(t.tx, t);
}

fn handle_withdrawal(t: Transaction, transactions: &mut Transactions, clients: &mut ClientList) {
    //first we check that the client account already exists, since we cannot withdraw from a client that doesn't have an account
    if let Some(client_acc) = clients.0.get_mut(&t.client) {
        //don't process any further if the client account is frozen
        if client_acc.locked { return }
        //we also need to make sure that the client has as much or more funds than he is trying to withdraw
        //unwrap() here is ok since withdrawal functions are also garaunteed to have an amount
        if client_acc.available >= t.amount.unwrap() {
            //reduce the available and total account funds by the withdrawal amount
            client_acc.available -= t.amount.unwrap();
            client_acc.total -= t.amount.unwrap();

            //add the transaction to the valid list since we processed it
            transactions.valid.insert(t.tx, t);
        }
    }
    //if the client account was not found or the client doesnt have enough funds, we throw out the transaction and do nothing
    //NOTICE that we do NOT add it to our valid list of transactions either
}

fn handle_dispute(t: Transaction, transactions: &mut Transactions, clients: &mut ClientList) {
    //A dispute consists of client either flagging an earlier valid deposit or withdrawal
    //Thus we wanna first make sure that the dispute is referring to a valid past transaction
    if let Some(past_tx) = transactions.valid.get_mut(&t.tx) {
        //if the transaction was valid, we can grab the client account associated with the transaction
        if let Some(client_acc) = clients.0.get_mut(&t.client) {
            //don't process any further if the client account is frozen
            if client_acc.locked { return }
            //now we decrease the client availabe and increase the client hold by the same amount of the disputed transaction
            client_acc.available -= past_tx.amount.unwrap();
            client_acc.held += past_tx.amount.unwrap();

            //make a valid dispute object that stores the amount being disuputed for easy access if we ever wanna resolve the dispute
            let d = DisputeAmt(past_tx.amount.unwrap());
            //insert the dispute into our map of disputes that can be accessed by dispute tx id
            transactions.disputes.insert(t.tx, d);
        }
    }
    //if we are disputing a transaction that doesn't exist or cant find the client account for the transaction, then we do nothing
    //NOTICE that we do NOT add to the disputes map either
}

fn handle_resolve(t: Transaction, transactions: &mut Transactions, clients: &mut ClientList) {
    //first we need to make sure that we are trying to resolve a past valid dispute
    if let Some(past_dispute) = transactions.disputes.get_mut(&t.tx) {
        //if valid dispute, then we get the clients account info
        if let Some(client_acc) = clients.0.get_mut(&t.client) {
            //don't process any further if the client account is frozen
            if client_acc.locked { return }
            //now we just need to update their availabe and held using the amt stored in the dispute
            client_acc.available += past_dispute.0;
            client_acc.held -= past_dispute.0;

            //now since we have resolved the dispute, we can remove it from the map of valid disputes
            //its ok to unwrap here as well since we have already made sure that the dispute exists in the first place
            transactions.disputes.remove(&t.tx).unwrap();
        }
    }
    //If we do not recognize a valid dispute to resolve, then we can throw this transaction out and do nothing
}

fn handle_chargeback(t: Transaction, transactions: &mut Transactions, clients: &mut ClientList) {
    //check if the chargeback is referring to a valid past dispute
    if let Some(past_dispute) = transactions.disputes.get_mut(&t.tx) {
        //if valid past dispute, get the clients account info
        if let Some(client_acc) = clients.0.get_mut(&t.client) {
            //don't process any further if the client account is frozen
            if client_acc.locked { return }
            //reduce the clients held and total amt by the disputed amt
            client_acc.held -= past_dispute.0;
            client_acc.total -= past_dispute.0;

            //mark their account as frozen
            client_acc.locked = true;

            //remove the disputed transaction
            transactions.disputes.remove(&t.tx).unwrap();
        }
    }
    //If the charageback refers to a dispute that does not exist, then simply throw this transactions out
}

fn print_client_info(clients: &ClientList) {
    println!("client, available, held, total, locked");
    //iterate through client accounts map and print pertinent info
    for (client_id, account)  in clients.0.iter() {
        println!("{:.4}, {:.4}, {:.4}, {:.4}, {}", client_id, account.available, account.held, account.total, account.locked);
    }
}