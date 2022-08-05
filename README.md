## Usage

#### Build </br>
`cargo b`

#### Run
Input file must be passed in as singular and only arg </br>
`cargo r -- test.csv`

The output can also be piped to another file instead of stdout like so </br>
`cargo r -- test.csv > accounts.csv`

## Assumptions
As the directions were faily open-ended on some parts, I made the following assumptions
```
- A valid singular csv file be passed in as the args
- The CSV file will follow the correct format
- The possible transaction types are limited to those described
- No transaction has any missing or extra data, such as a Deposit not having an amount field
- It is only possible to dispute a previous valid deposit or withdrawal
- It is only possible to resolve a previous valid dispute
- It is only possible to issue a chargeback on a previous valid dispute
```

## Error Handling
While in a large project, it may be beneficial to use custom error types, I found the built in crate and std error types sufficient for my use. Possible reasons for errors include:
```
- Passing in an unvalid input file
- Input file exists in wrong directory
- The format of the input file does not follow specification and thus cannot be properly deserialized
- The input is not valid UTF-8, which may cause an error as we stream the CSV into StringRecords
```
Transaction based errors should not be a proplem, since these are handled logically and any invalid transaction is automatically thrown out. </br>
I did not introduce any extra unsafe code either.

## Testing
I mostly hand tested my program using some different sample inputs that I came up with, since none were provided. I included some below </br>

### Test 1: </br>
Input
```
type, client, tx, amount
deposit, 1, 1, 1.0
deposit, 2, 2, 2.0
deposit, 1, 3, 2.0
withdrawal, 1, 4, 1.5
withdrawal, 2, 5, 3.0
```
Output
```
client, available, held, total, locked
1, 1.5000, 0.0000, 1.5000, false
2, 2.0000, 0.0000, 2.0000, false
```
### Test 2: </br>
Input

```
type, client, tx, amount
deposit, 1, 1, 1.0
deposit, 1, 2, 20
withdrawal, 1, 3, 14
dispute, 1, 1
```

Output
```
client, available, held, total, locked
1, 6.0000, 1.0000, 7.0000, false
```
### Test 3: </br>
Input
```
type, client, tx, amount
deposit, 1, 1, 1.0
deposit, 1, 2, 20
deposit, 2, 3, 10
deposit, 2, 4, 5
withdrawal, 2, 5, 2
dispute, 1, 1
dispute, 2, 4
resolve, 1, 1
chargeback, 2, 4
```

Output
```
client, available, held, total, locked
1, 21.0000, 0.0000, 21.0000, false
2, 8.0000, 0.0000, 8.0000, true
```