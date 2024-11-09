// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.13;

import "forge-std/Test.sol";
import "../src/Quartz.sol";
import "@automata-dcap/interfaces/IAttestation.sol";

contract QuartzTest is Test {
    bytes32 dummyMrEnclave = bytes32("dummyMrEnclave");
    string dummyChainID = "dummyChainID";
    uint256 dummyTrustedHeight = 100;
    bytes32 dummyTrustedHash = bytes32("dummyTrustedHash");
    address dummyPccs = address(0x1234567890123456789012345678901234567890);
    bytes dummyQuote =
        hex"03000200000000000b001000939a7233f79c4ca9940a0db3957f06077944f37bdafec57cf7d4ab6bc395e0a1000000000e0e100fffff0100000000000000000000000000000000000000000000000000000000000000000000000000000000000500000000000000e70000000000000081f36e827391dc0916f06215400b32a5b2823c4c3349428a50dbc531cfdad5a40000000000000000000000000000000000000000000000000000000000000000255197a6388e504446dbf83726c2a9cb3cef9035cc3dabd6cf47d69a994f95940000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000ca100000dd653050dd320cc49517a0b858fad3587e4050121e074c671e5d396a0acb402dc0d01648efa8ac87f47a1abb66a45c664dfad325c40628037303d6821f8c67d453b560130d79fd96a0508e1df00f67064f302ea0e1c8226764a5fc7fe5bc8c88aa8fe5d18a064a2ceb780cefa8b5daff5346516784b11dac7464bc641cbf16810e0e100fffff0100000000000000000000000000000000000000000000000000000000000000000000000000000000001500000000000000e70000000000000078fe8cfd01095a0f108aff5c40624b93612d6c28b73e1a8d28179c9ddf0e068600000000000000000000000000000000000000000000000000000000000000008c4f5775d796503e96137f77c68a829a0056ac8ded70140b081b094490c57bff00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001000b000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000008c8b256a3f090a680d3afb94dbd79aa100f305368b3ed48fcacfe80bec2bef580000000000000000000000000000000000000000000000000000000000000000964c950f663a505a471573a7b5ad7f80f76292568231cfc6552dcef204459cae9dda1b5bfa1379620fb893e6326a35473d55724373127d6620d693e54cc05dc22000000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f0500620e00002d2d2d2d2d424547494e2043455254494649434154452d2d2d2d2d0a4d494945386a4343424a696741774942416749554854525a7054426e4d59655063575531354542316643507561485177436759494b6f5a497a6a3045417749770a634445694d434147413155454177775a535735305a577767553064594946424453794251624746305a6d397962534244515445614d42674741315545436777520a535735305a577767513239796347397959585270623234784644415342674e564241634d43314e68626e526849454e7359584a684d51737743515944565151490a44414a445154454c4d416b474131554542684d4356564d774868634e4d6a51774e5445344d5449314e4451325768634e4d7a45774e5445344d5449314e4451320a576a42774d534977494159445651514444426c4a626e526c624342545231676755454e4c49454e6c636e52705a6d6c6a5958526c4d526f77474159445651514b0a4442464a626e526c6243424462334a7762334a6864476c76626a45554d424947413155454277774c553246756447456751327868636d4578437a414a42674e560a4241674d416b4e424d517377435159445651514745774a56557a425a4d424d4742797147534d34394167454743437147534d34394177454841304941424f64700a696252626a7639566159476a7159584a766f7670333253752f594861313541727943546735566c384762744348417a396e5952786d4e6a303372553548687a4d0a513030752b364a6d794748744b4f773866364f6a67674d4f4d494944436a416642674e5648534d4547444157674253566231334e765276683655424a796454300a4d383442567776655644427242674e56485238455a4442694d47436758714263686c706f64485277637a6f764c32467761533530636e567a6447566b633256790a646d6c6a5a584d75615735305a577775593239744c334e6e6543396a5a584a3061575a7059324630615739754c3359304c33426a61324e796244396a595431770a624746305a6d397962535a6c626d4e765a476c755a7a316b5a584977485159445652304f42425945464167706b386e6b7a4c6371776b6749376f7567574844560a574d67314d41344741315564447745422f775145417749477744414d42674e5648524d4241663845416a41414d4949434f77594a4b6f5a496876684e415130420a424949434c444343416967774867594b4b6f5a496876684e415130424151515151576f736c643645563066526f62747368385149737a434341575547436971470a534962345451454e41514977676746564d42414743797147534962345451454e415149424167454f4d42414743797147534962345451454e415149434167454f0a4d42414743797147534962345451454e41514944416745444d42414743797147534962345451454e41514945416745444d42454743797147534962345451454e0a41514946416749412f7a415242677371686b69472b4530424451454342674943415038774541594c4b6f5a496876684e4151304241676343415145774541594c0a4b6f5a496876684e4151304241676743415141774541594c4b6f5a496876684e4151304241676b43415141774541594c4b6f5a496876684e4151304241676f430a415141774541594c4b6f5a496876684e4151304241677343415141774541594c4b6f5a496876684e4151304241677743415141774541594c4b6f5a496876684e0a4151304241673043415141774541594c4b6f5a496876684e4151304241673443415141774541594c4b6f5a496876684e4151304241673843415141774541594c0a4b6f5a496876684e4151304241684143415141774541594c4b6f5a496876684e4151304241684543415130774877594c4b6f5a496876684e41513042416849450a4541344f4177502f2f7745414141414141414141414141774541594b4b6f5a496876684e4151304241775143414141774641594b4b6f5a496876684e415130420a4241514741474271414141414d41384743697147534962345451454e4151554b415145774867594b4b6f5a496876684e4151304242675151446758512b3446660a2b6c2b4853522f457161474d737a424542676f71686b69472b453042445145484d4459774541594c4b6f5a496876684e4151304242774542416638774541594c0a4b6f5a496876684e4151304242774942415141774541594c4b6f5a496876684e4151304242774d4241514177436759494b6f5a497a6a304541774944534141770a52514968414c3047436752526b30764e6c585a594e506d5738634f313632364c4353332f2f4c6d6f416638756a4457484169426d41324d56347058774f386d6d0a4171444e4c345a6843792f64657a4842796c746f307271377149664c51773d3d0a2d2d2d2d2d454e442043455254494649434154452d2d2d2d2d0a2d2d2d2d2d424547494e2043455254494649434154452d2d2d2d2d0a4d4949436c6a4343416a32674177494241674956414a567658633239472b487051456e4a3150517a7a674658433935554d416f4743437147534d343942414d430a4d476778476a415942674e5642414d4d45556c756447567349464e48574342536232393049454e424d526f77474159445651514b4442464a626e526c624342440a62334a7762334a6864476c76626a45554d424947413155454277774c553246756447456751327868636d4578437a414a42674e564241674d416b4e424d5173770a435159445651514745774a56557a4165467730784f4441314d6a45784d4455774d5442614677307a4d7a41314d6a45784d4455774d5442614d484178496a41670a42674e5642414d4d47556c756447567349464e4857434251513073675547786864475a76636d306751304578476a415942674e5642416f4d45556c75644756730a49454e76636e4276636d4630615739754d5251774567594456515148444174545957353059534244624746795954454c4d416b474131554543417743513045780a437a414a42674e5642415954416c56544d466b77457759484b6f5a497a6a3043415159494b6f5a497a6a304441516344516741454e53422f377432316c58534f0a3243757a7078773734654a423732457944476757357258437478327456544c7136684b6b367a2b5569525a436e71523770734f766771466553786c6d546c4a6c0a65546d693257597a33714f42757a43427544416642674e5648534d4547444157674251695a517a575770303069664f44744a5653763141624f536347724442530a42674e5648523845537a424a4d45656752614244686b466f64485277637a6f764c324e6c636e52705a6d6c6a5958526c63793530636e567a6447566b633256790a646d6c6a5a584d75615735305a577775593239744c306c756447567355306459556d397664454e424c6d526c636a416442674e5648513445466751556c5739640a7a62306234656c4153636e553944504f4156634c336c517744675944565230504151482f42415144416745474d42494741315564457745422f7751494d4159420a4166384341514177436759494b6f5a497a6a30454177494452774177524149675873566b6930772b6936565947573355462f32327561586530594a446a3155650a6e412b546a44316169356343494359623153416d4435786b66545670766f34556f79695359787244574c6d5552344349394e4b7966504e2b0a2d2d2d2d2d454e442043455254494649434154452d2d2d2d2d0a2d2d2d2d2d424547494e2043455254494649434154452d2d2d2d2d0a4d4949436a7a4343416a53674177494241674955496d554d316c71644e496e7a6737535655723951477a6b6e42717777436759494b6f5a497a6a3045417749770a614445614d4267474131554541777752535735305a5777675530645949464a766233516751304578476a415942674e5642416f4d45556c756447567349454e760a636e4276636d4630615739754d5251774567594456515148444174545957353059534244624746795954454c4d416b47413155454341774351304578437a414a0a42674e5642415954416c56544d423458445445344d4455794d5445774e4455784d466f58445451354d54497a4d54497a4e546b314f566f77614445614d4267470a4131554541777752535735305a5777675530645949464a766233516751304578476a415942674e5642416f4d45556c756447567349454e76636e4276636d46300a615739754d5251774567594456515148444174545957353059534244624746795954454c4d416b47413155454341774351304578437a414a42674e56424159540a416c56544d466b77457759484b6f5a497a6a3043415159494b6f5a497a6a3044415163445167414543366e45774d4449595a4f6a2f69505773437a61454b69370a314f694f534c52466857476a626e42564a66566e6b59347533496a6b4459594c304d784f346d717379596a6c42616c54565978465032734a424b357a6c4b4f420a757a43427544416642674e5648534d4547444157674251695a517a575770303069664f44744a5653763141624f5363477244425342674e5648523845537a424a0a4d45656752614244686b466f64485277637a6f764c324e6c636e52705a6d6c6a5958526c63793530636e567a6447566b63325679646d6c6a5a584d75615735300a5a577775593239744c306c756447567355306459556d397664454e424c6d526c636a416442674e564851344546675155496d554d316c71644e496e7a673753560a55723951477a6b6e4271777744675944565230504151482f42415144416745474d42494741315564457745422f7751494d4159424166384341514577436759490a4b6f5a497a6a3045417749445351417752674968414f572f35516b522b533943695344634e6f6f774c7550524c735747662f59693747535839344267775477670a41694541344a306c72486f4d732b586f356f2f7358364f39515778485241765a55474f6452513763767152586171493d0a2d2d2d2d2d454e442043455254494649434154452d2d2d2d2d0a00"; // Replace with appropriate format if needed
    bytes invalidQuote = hex"99";

    Quartz.Config dummyConfig;
    Quartz quartz;

    event SessionCreated(address indexed quartz);
    event PubKeySet(bytes32 indexed enclavePubKey);

    function setUp() public {
        console.log("Test Suite started!!");

        // Set up the dummy LightClientOpts
        Quartz.LightClientOpts memory lightClientOpts = Quartz.LightClientOpts({
            chainID: dummyChainID,
            trustedHeight: dummyTrustedHeight,
            trustedHash: dummyTrustedHash
        });

        // Set up the dummy Config
        dummyConfig = Quartz.Config({mrEnclave: dummyMrEnclave, lightClientOpts: lightClientOpts, pccs: dummyPccs});
    }

    function testDeployContract_Success() public {
        // Deploy the Quartz contract and store the address
        quartz = new Quartz(dummyConfig, dummyQuote);

        vm.expectEmit(true, true, false, false);
        emit SessionCreated(address(quartz)); // TODO - this test is failing, but it is working as intended

        // Check that the config is stored correctly
        (bytes32 mrEnclave, Quartz.LightClientOpts memory lightClientOpts, address pccs) = quartz.config();
        assertEq(mrEnclave, dummyConfig.mrEnclave);
        assertEq(lightClientOpts.chainID, dummyConfig.lightClientOpts.chainID);
        assertEq(lightClientOpts.trustedHeight, dummyConfig.lightClientOpts.trustedHeight);
        assertEq(lightClientOpts.trustedHash, dummyConfig.lightClientOpts.trustedHash);
        assertEq(pccs, dummyConfig.pccs);
    }

    function testDeployContract_Failure() public {
        // Expect revert due to failed attestation
        vm.expectRevert(); // TODO - maybe test the revert message
        quartz = new Quartz(dummyConfig, invalidQuote);
    }

    function testSetSessionPubKey_Success() public {
        quartz = new Quartz(dummyConfig, dummyQuote);

        bytes32 dummyPubKey = bytes32("dummyPublicKey");

        vm.expectEmit(true, true, true, true);
        emit PubKeySet(dummyPubKey);

        // Call setSessionPubKey and check if it succeeds
        quartz.setSessionPubKey(dummyPubKey, dummyQuote);

        // Verify that the public key is stored
        assertEq(quartz.enclavePubKey(), dummyPubKey);
    }

    function testSetSessionPubKey_Failure() public {
        quartz = new Quartz(dummyConfig, dummyQuote);

        // Expect revert due to failed attestation
        vm.expectRevert(); // TODO - maybe test the revert message
        quartz.setSessionPubKey(bytes32("dummyPublicKey"), invalidQuote);
    }
}
